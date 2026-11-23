mod model;
mod platform;
mod reminder_lifetime;
mod scheduler;
mod sedentary;
mod storage;

use model::{Daily, Saved, Settings, Snapshot, clock};
use reminder_lifetime::{CardKind, ReminderCard, ReminderLifetime};
use scheduler::Scheduler;
use sedentary::SedentaryTimer;
use std::{
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{
    Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::ManagerExt;

struct Runtime {
    settings: Settings,
    countdown_visible: bool,
    scheduler: Scheduler,
    sedentary: SedentaryTimer,
    waiting_for_desktop: Option<i64>,
    warning: Option<String>,
    path: PathBuf,
}

impl Runtime {
    fn persist(&mut self) -> Result<(), String> {
        let saved = Saved {
            settings: self.settings.clone(),
            daily: Daily::from(&self.scheduler),
            countdown_visible: self.countdown_visible,
        };
        let data = serde_json::to_string_pretty(&saved).map_err(|e| e.to_string())?;
        storage::save(&self.path, &data).map_err(|e| format!("无法保存设置：{e}"))
    }
    fn snapshot(&self) -> Snapshot {
        self.snapshot_at(clock())
    }

    fn snapshot_at(&self, now: scheduler::Clock) -> Snapshot {
        let phase = if self.scheduler.paused {
            "paused"
        } else if !self.settings.days[now.weekday] {
            "dayOff"
        } else if self.scheduler.next.is_none()
            || now.second
                >= u32::from(self.settings.schedule().expect("validated settings").end) * 60
        {
            "finished"
        } else if self.scheduler.next.is_some_and(|n| n <= now.timestamp) {
            "due"
        } else {
            "scheduled"
        };
        Snapshot {
            settings: self.settings.clone(),
            countdown_visible: self.countdown_visible,
            paused: self.scheduler.paused,
            phase: phase.into(),
            next_at: self.scheduler.next,
            warning: self.warning.clone(),
            config_path: self.path.display().to_string(),
        }
    }
}

type AppState = Mutex<Runtime>;

const COUNTDOWN_WIDTH: f64 = 164.;
const COUNTDOWN_HEIGHT: f64 = 52.;
const REMINDER_WIDTH: f64 = 320.;
const REMINDER_HEIGHT: f64 = 164.;

fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_reminder(app: &tauri::AppHandle) {
    if let Ok(mut lifetime) = app.state::<Mutex<ReminderLifetime>>().lock() {
        lifetime.clear();
    }
    if let Some(window) = app.get_webview_window("reminder") {
        let _ = window.hide();
    }
}

fn place_countdown(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    if let Some(monitor) = window.primary_monitor()? {
        let area = monitor.work_area();
        let scale = monitor.scale_factor();
        window.set_position(tauri::PhysicalPosition::new(
            area.position.x
                + (f64::from(area.size.width) - (COUNTDOWN_WIDTH + 20.) * scale).max(0.) as i32,
            area.position.y + (20. * scale) as i32,
        ))?;
    }
    Ok(())
}

fn change_countdown_visibility(
    app: &tauri::AppHandle,
    visible: Option<bool>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("countdown")
        .ok_or_else(|| "无法找到倒计时悬浮窗".to_string())?;
    let state = app.state::<AppState>();
    let mut runtime = state.lock().map_err(|e| e.to_string())?;
    let previous = runtime.countdown_visible;
    let visible = visible.unwrap_or(!previous);
    if visible != previous {
        runtime.countdown_visible = visible;
        if let Err(error) = runtime.persist() {
            runtime.countdown_visible = previous;
            return Err(error);
        }
    }
    drop(runtime);
    let shown = if visible {
        window.show()
    } else {
        window.hide()
    };
    app.emit("state-changed", ()).map_err(|e| e.to_string())?;
    shown.map_err(|e| e.to_string())
}

#[tauri::command]
fn set_countdown_visibility(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    change_countdown_visibility(&app, Some(visible))
}

#[tauri::command]
fn move_countdown(window: tauri::WebviewWindow, x: f64, y: f64) -> Result<(), String> {
    if window.label() != "countdown"
        || !(0. ..=COUNTDOWN_WIDTH).contains(&x)
        || !(0. ..=COUNTDOWN_HEIGHT).contains(&y)
    {
        return Err("无效的倒计时窗口拖动位置".into());
    }
    let cursor = window.cursor_position().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    window
        .set_position(tauri::PhysicalPosition::new(
            (cursor.x - x * scale).round() as i32,
            (cursor.y - y * scale).round() as i32,
        ))
        .map_err(|e| e.to_string())
}

fn show_reminder(
    app: &tauri::AppHandle,
    settings: &Settings,
    kind: CardKind,
) -> Result<(), String> {
    let state = app.state::<Mutex<ReminderLifetime>>();
    let mut lifetime = state.lock().map_err(|e| e.to_string())?;
    if lifetime
        .card()
        .is_some_and(|card| card.kind == CardKind::Offwork)
    {
        return Err("正式提醒正在显示，请稍后再预览".into());
    }
    let window = app
        .get_webview_window("reminder")
        .ok_or("无法找到提醒窗口")?;
    let monitor = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten())
        .or(window.primary_monitor().map_err(|e| e.to_string())?);
    if let Some(monitor) = monitor {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let size = &area.size;
        let pos = &area.position;
        window
            .set_position(tauri::PhysicalPosition::new(
                pos.x + (f64::from(size.width) - (REMINDER_WIDTH + 20.) * scale).max(0.) as i32,
                pos.y + (f64::from(size.height) - (REMINDER_HEIGHT + 20.) * scale).max(0.) as i32,
            ))
            .map_err(|e| e.to_string())?;
    }
    window.show().map_err(|e| e.to_string())?;
    let deadline = lifetime.show(Instant::now(), settings, kind);
    if let Err(error) = app.emit("reminder-shown", lifetime.card()) {
        lifetime.clear();
        let _ = window.hide();
        return Err(error.to_string());
    }
    drop(lifetime);
    schedule_reminder_hide(app, deadline);
    if settings.sound {
        platform::play_sound();
    }
    Ok(())
}

fn schedule_reminder_hide(app: &tauri::AppHandle, deadline: Instant) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
        let handler = app.clone();
        let _ = app.run_on_main_thread(move || {
            let should_hide = handler
                .state::<Mutex<ReminderLifetime>>()
                .lock()
                .is_ok_and(|lifetime| lifetime.should_hide(deadline, Instant::now()));
            if should_hide {
                hide_reminder(&handler);
            }
        });
    });
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let runtime = state.lock().map_err(|e| e.to_string())?;
    Ok(runtime.snapshot())
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    settings: Settings,
    state: State<'_, AppState>,
) -> Result<Snapshot, String> {
    settings.schedule()?;
    let mut runtime = state.lock().map_err(|e| e.to_string())?;
    let previous_settings = runtime.settings.clone();
    let previous_schedule = runtime.scheduler.clone();
    let previous_autostart = app
        .autolaunch()
        .is_enabled()
        .map_err(|e| format!("无法读取自动启动状态：{e}"))?;
    if settings.autostart {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| format!("无法修改自动启动：{e}"))?;
    runtime.settings = settings;
    runtime.scheduler.reconfigure();
    let schedule = runtime.settings.schedule()?;
    runtime.scheduler.tick(&schedule, clock(), |_| false);
    if let Err(error) = runtime.persist() {
        runtime.settings = previous_settings;
        runtime.scheduler = previous_schedule;
        let rollback = if previous_autostart {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };
        return Err(match rollback {
            Ok(_) => error,
            Err(e) => format!("{error}；自动启动恢复失败：{e}"),
        });
    }
    timing_log(
        &runtime.path,
        format_args!("设置已保存，下班时间 {}", runtime.settings.start_time),
    );
    runtime.warning = None;
    let snapshot = runtime.snapshot();
    drop(runtime);
    hide_reminder(&app);
    app.emit("state-changed", ()).map_err(|e| e.to_string())?;
    Ok(snapshot)
}

fn change_reminder(app: &tauri::AppHandle, action: &str) -> Result<Snapshot, String> {
    let state = app.state::<AppState>();
    let mut runtime = state.lock().map_err(|e| e.to_string())?;
    let now = clock();
    let schedule = runtime.settings.schedule()?;
    let previous = runtime.scheduler.clone();
    match action {
        "pause" => runtime.scheduler.pause(now),
        "resume" => runtime.scheduler.resume(&schedule, now),
        "snooze" => runtime.scheduler.snooze(&schedule, now),
        _ => return Err("未知操作".into()),
    }
    if let Err(error) = runtime.persist() {
        runtime.scheduler = previous;
        return Err(error);
    }
    timing_log(&runtime.path, format_args!("提醒操作 {action}"));
    let snapshot = runtime.snapshot();
    drop(runtime);
    hide_reminder(app);
    app.emit("state-changed", ()).map_err(|e| e.to_string())?;
    Ok(snapshot)
}

#[tauri::command]
fn reminder_action(app: tauri::AppHandle, action: String) -> Result<Snapshot, String> {
    change_reminder(&app, &action)
}

#[tauri::command]
fn test_reminder(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.lock().map_err(|e| e.to_string())?.settings.clone();
    show_reminder(&app, &settings, CardKind::Preview)
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    show_settings(&app);
}

#[tauri::command]
fn get_reminder_card(
    state: State<'_, Mutex<ReminderLifetime>>,
) -> Result<Option<ReminderCard>, String> {
    Ok(state.lock().map_err(|e| e.to_string())?.card().cloned())
}

#[tauri::command]
fn card_action(app: tauri::AppHandle, id: u32, action: String) -> Result<(), String> {
    let state = app.state::<Mutex<ReminderLifetime>>();
    let action = state.lock().map_err(|e| e.to_string())?.action(id, &action);
    match action {
        None => Ok(()),
        Some("dismiss") => {
            hide_reminder(&app);
            Ok(())
        }
        Some(action) => change_reminder(&app, action).map(|_| ()),
    }
}

fn timing_log(settings_path: &std::path::Path, message: std::fmt::Arguments<'_>) {
    use std::io::Write;
    let path = settings_path.with_file_name("reminder-timing.log");
    let full = path.metadata().is_ok_and(|m| m.len() >= 128 * 1024);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(full)
        .append(!full)
        .open(path)
    {
        let _ = writeln!(
            file,
            "{} {message}",
            chrono::Local::now().format("%F %T%.3f")
        );
    }
}

fn log_error(app: &tauri::AppHandle, message: String) {
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(mut runtime) = state.lock()
    {
        runtime.warning = Some(message.clone());
    }
    if let Ok(directory) = app.path().app_data_dir() {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join("offwork.log"))
        {
            let _ = writeln!(file, "{} {message}", chrono::Local::now().to_rfc3339());
        }
    }
}

// Run the check and the actual display together on the window thread so a
// queued check observes any intervening pause, settings change or cutoff.
fn poll_reminders(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let Ok(mut runtime) = state.lock() else {
        return;
    };
    let Ok(schedule) = runtime.settings.schedule() else {
        return;
    };
    let Runtime {
        scheduler,
        sedentary,
        path,
        waiting_for_desktop,
        settings,
        ..
    } = &mut *runtime;
    let before = Daily::from(&*scheduler);
    let mut delivery_error = None;
    let now = clock();
    scheduler.tick(&schedule, now, |due| {
        let scheduled = chrono::DateTime::from_timestamp(due, 0)
            .map(|t| t.with_timezone(&chrono::Local).format("%F %T").to_string())
            .unwrap_or_else(|| due.to_string());
        if !platform::desktop_available() {
            if *waiting_for_desktop != Some(due) {
                timing_log(
                    path,
                    format_args!("计划 {scheduled}，等待桌面解锁或恢复可用"),
                );
                *waiting_for_desktop = Some(due);
            }
            return false;
        }
        let result = show_reminder(app, settings, CardKind::Offwork);
        if result.is_ok() {
            let gap = before
                .last_clock
                .map_or(0, |last| now.timestamp.saturating_sub(last));
            timing_log(
                path,
                format_args!("下班卡片已显示，计划 {scheduled}，距上次检查 {gap} 秒"),
            );
            *waiting_for_desktop = None;
        }
        delivery_error = result.err();
        delivery_error.is_none()
    });
    sedentary.tick(
        now,
        &schedule,
        settings.sedentary_minutes,
        platform::desktop_available,
        || {
            delivery_error = show_reminder(app, settings, CardKind::Sedentary).err();
            delivery_error.is_none()
        },
    );
    if (
        before.date,
        before.paused,
        before.next,
        before.last_shown,
        before.last_offset,
    ) != (
        scheduler.date,
        scheduler.paused,
        scheduler.next,
        scheduler.last_shown,
        scheduler.last_offset,
    ) && let Err(error) = runtime.persist()
    {
        runtime.warning = Some(error);
    }
    if let Some(error) = delivery_error
        && runtime.warning.as_ref() != Some(&error)
    {
        drop(runtime);
        log_error(app, error);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_settings(app)
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--autostart"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            save_settings,
            set_countdown_visibility,
            move_countdown,
            reminder_action,
            test_reminder,
            open_settings,
            card_action,
            get_reminder_card
        ])
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("settings.json");
            let (saved, warning) = match storage::load(&path) {
                Ok(Some(data)) => match serde_json::from_str::<Saved>(&data).and_then(|saved| {
                    saved
                        .settings
                        .schedule()
                        .map(|_| saved)
                        .map_err(serde::de::Error::custom)
                }) {
                    Ok(saved) => (saved, None),
                    Err(error) => {
                        let backup = path.with_extension(format!(
                            "invalid-{}.json",
                            chrono::Local::now().timestamp()
                        ));
                        std::fs::rename(&path, backup)?;
                        (
                            Saved::default(),
                            Some(format!("原设置无法读取，已备份并恢复默认设置：{error}")),
                        )
                    }
                },
                Ok(None) => (Saved::default(), None),
                Err(error) => return Err(error.into()),
            };
            let mut runtime = Runtime {
                scheduler: saved.daily.restore(),
                sedentary: SedentaryTimer::default(),
                settings: saved.settings,
                countdown_visible: saved.countdown_visible,
                path,
                waiting_for_desktop: None,
                warning,
            };
            match app.autolaunch().is_enabled() {
                Ok(enabled) => runtime.settings.autostart = enabled,
                Err(error) => runtime.warning = Some(format!("无法读取自动启动状态：{error}")),
            }
            let schedule = runtime.settings.schedule()?;
            runtime.scheduler.tick(&schedule, clock(), |_| false);
            runtime.persist()?;
            timing_log(
                &runtime.path,
                format_args!("程序启动，下班时间 {}", runtime.settings.start_time),
            );
            let countdown_visible = runtime.countdown_visible;
            app.manage(Mutex::new(runtime));
            app.manage(Mutex::new(ReminderLifetime::default()));

            // Prepare the hidden card during setup so synchronous commands never
            // create WebViews while the UI thread is servicing IPC.
            WebviewWindowBuilder::new(
                app,
                "reminder",
                WebviewUrl::App("index.html?view=reminder".into()),
            )
            .title("下班啦 · 提醒")
            .inner_size(REMINDER_WIDTH, REMINDER_HEIGHT)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .build()?;

            let countdown = WebviewWindowBuilder::new(
                app,
                "countdown",
                WebviewUrl::App("index.html?view=countdown".into()),
            )
            .title("下班与加班计时")
            .inner_size(COUNTDOWN_WIDTH, COUNTDOWN_HEIGHT)
            .background_color(tauri::window::Color(32, 29, 53, 255))
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .build()?;
            place_countdown(&countdown)?;
            if countdown_visible {
                countdown.show()?;
            }

            let open = MenuItem::with_id(app, "open", "打开设置", true, None::<&str>)?;
            let toggle = MenuItem::with_id(
                app,
                "countdown",
                "显示 / 隐藏下班倒计时",
                true,
                None::<&str>,
            )?;
            let test = MenuItem::with_id(app, "test", "测试提醒", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "今天不再提醒", true, None::<&str>)?;
            let resume = MenuItem::with_id(app, "resume", "恢复今日提醒", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出下班啦", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &toggle, &test, &pause, &resume, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("bundled icon").clone())
                .tooltip("下班啦 · 工作有时，生活也是")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_settings(app),
                    "countdown" => {
                        if let Err(error) = change_countdown_visibility(app, None) {
                            log_error(app, error);
                        }
                    }
                    "test" => {
                        if let Err(error) = test_reminder(app.clone(), app.state()) {
                            log_error(app, error);
                        }
                    }
                    "pause" | "resume" => {
                        if let Err(e) = change_reminder(app, event.id.as_ref()) {
                            log_error(app, e);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_settings(tray.app_handle());
                    }
                })
                .build(app)?;
            if !std::env::args().any(|arg| arg == "--autostart") {
                show_settings(app.handle());
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    let next = handle
                        .state::<AppState>()
                        .lock()
                        .ok()
                        .and_then(|r| r.scheduler.next);
                    let wall_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default();
                    std::thread::sleep(poll_delay(wall_time, next));
                    let app = handle.clone();
                    let (done, finished) = std::sync::mpsc::channel();
                    if handle
                        .run_on_main_thread(move || {
                            poll_reminders(&app);
                            let _ = done.send(());
                        })
                        .is_err()
                        || finished.recv().is_err()
                    {
                        break;
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "countdown" {
                    if let Err(error) =
                        change_countdown_visibility(window.app_handle(), Some(false))
                    {
                        log_error(window.app_handle(), error);
                    }
                } else if window.label() == "reminder" {
                    hide_reminder(window.app_handle());
                } else {
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("无法启动下班啦");
}

fn poll_delay(wall_time: Duration, next: Option<i64>) -> Duration {
    // Keep the fallback aligned to the clock as well: a newly saved HH:MM
    // deadline can arrive while the worker is already asleep.
    let phase = Duration::new(wall_time.as_secs() % 2, wall_time.subsec_nanos());
    let heartbeat = Duration::from_secs(2) - phase;
    next.and_then(|timestamp| u64::try_from(timestamp).ok())
        .and_then(|seconds| Duration::from_secs(seconds).checked_sub(wall_time))
        .filter(|delay| !delay.is_zero())
        .map_or(heartbeat, |delay| delay.min(heartbeat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polling_wakes_at_the_scheduled_second_even_after_a_slow_previous_check() {
        let before = Duration::from_millis(59_800);
        assert_eq!(
            before + poll_delay(before, Some(60)),
            Duration::from_secs(60)
        );
        let before_repeat = Duration::from_millis(120_300);
        assert_eq!(
            before_repeat + poll_delay(before_repeat, Some(121)),
            Duration::from_secs(121)
        );
    }

    #[test]
    fn locked_or_paused_reminders_keep_a_bounded_positive_poll_delay() {
        let now = Duration::from_secs(60);
        for next in [None, Some(59), Some(60), Some(3600)] {
            assert_eq!(poll_delay(now, next), Duration::from_secs(2));
        }
        assert_eq!(
            poll_delay(Duration::from_millis(59_800), None),
            Duration::from_millis(200)
        );
    }

    #[test]
    fn last_reminder_of_the_day_finishes_the_schedule() {
        let mut runtime = Runtime {
            settings: Settings {
                start_time: "00:00".into(),
                end_time: "23:59".into(),
                interval_minutes: 1440,
                days: [true; 7],
                ..Settings::default()
            },
            scheduler: Scheduler::default(),
            sedentary: SedentaryTimer::default(),
            countdown_visible: true,
            waiting_for_desktop: None,
            warning: None,
            path: PathBuf::new(),
        };
        let midnight = scheduler::Clock {
            date: 1,
            weekday: 0,
            second: 0,
            timestamp: 86400,
            offset: 0,
        };
        assert!(
            runtime
                .scheduler
                .tick(&runtime.settings.schedule().unwrap(), midnight, |_| true)
        );
        assert_eq!(runtime.snapshot_at(midnight).phase, "finished");
    }
}
