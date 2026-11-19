mod model;
mod platform;
mod scheduler;
mod storage;

use model::{Daily, Saved, Settings, Snapshot, clock};
use scheduler::Scheduler;
use std::{path::PathBuf, sync::Mutex, time::Duration};
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
        let now = clock();
        let phase = if self.scheduler.paused {
            "paused"
        } else if !self.settings.days[now.weekday] {
            "dayOff"
        } else if now.second
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

fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_reminder(app: &tauri::AppHandle) {
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

fn show_reminder(app: &tauri::AppHandle, sound: bool) -> tauri::Result<()> {
    let window = app
        .get_webview_window("reminder")
        .ok_or(tauri::Error::WindowNotFound)?;
    let monitor = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten())
        .or(window.primary_monitor()?);
    if let Some(monitor) = monitor {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let size = &area.size;
        let pos = &area.position;
        window.set_position(tauri::PhysicalPosition::new(
            pos.x + (f64::from(size.width) - 440. * scale).max(0.) as i32,
            pos.y + (f64::from(size.height) - 340. * scale).max(0.) as i32,
        ))?;
    }
    window.show()?;
    app.emit("reminder-shown", ())?;
    if sound {
        platform::play_sound();
    }
    Ok(())
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
    runtime.scheduler.tick(&schedule, clock(), false);
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
    let sound = state.lock().map_err(|e| e.to_string())?.settings.sound;
    show_reminder(&app, sound).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    show_settings(&app);
}

#[tauri::command]
fn dismiss_reminder(app: tauri::AppHandle) {
    hide_reminder(&app);
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
            dismiss_reminder
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
                settings: saved.settings,
                countdown_visible: saved.countdown_visible,
                path,
                warning,
            };
            match app.autolaunch().is_enabled() {
                Ok(enabled) => runtime.settings.autostart = enabled,
                Err(error) => runtime.warning = Some(format!("无法读取自动启动状态：{error}")),
            }
            let schedule = runtime.settings.schedule()?;
            runtime.scheduler.tick(&schedule, clock(), false);
            runtime.persist()?;
            let countdown_visible = runtime.countdown_visible;
            app.manage(Mutex::new(runtime));

            // Prepare the hidden card during setup so synchronous commands never
            // create WebViews while the UI thread is servicing IPC.
            WebviewWindowBuilder::new(
                app,
                "reminder",
                WebviewUrl::App("index.html?view=reminder".into()),
            )
            .title("下班啦 · 提醒")
            .inner_size(420., 320.)
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
                        let sound = app
                            .state::<AppState>()
                            .lock()
                            .map(|r| r.settings.sound)
                            .unwrap_or(false);
                        if let Err(e) = show_reminder(app, sound) {
                            log_error(app, e.to_string());
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
                    std::thread::sleep(Duration::from_secs(2));
                    let card_open = handle
                        .get_webview_window("reminder")
                        .is_some_and(|w| w.is_visible().unwrap_or(false));
                    let show = {
                        let state = handle.state::<AppState>();
                        let mut runtime = match state.lock() {
                            Ok(r) => r,
                            Err(_) => continue,
                        };
                        let schedule = match runtime.settings.schedule() {
                            Ok(c) => c,
                            Err(_) => continue,
                        };
                        let before = (
                            runtime.scheduler.date,
                            runtime.scheduler.paused,
                            runtime.scheduler.next,
                            runtime.scheduler.last_shown,
                            runtime.scheduler.last_offset,
                        );
                        let show = runtime.scheduler.tick(
                            &schedule,
                            clock(),
                            !card_open && platform::desktop_available(),
                        );
                        if before
                            != (
                                runtime.scheduler.date,
                                runtime.scheduler.paused,
                                runtime.scheduler.next,
                                runtime.scheduler.last_shown,
                                runtime.scheduler.last_offset,
                            )
                            && let Err(error) = runtime.persist()
                        {
                            runtime.warning = Some(error);
                        }
                        show.then_some(runtime.settings.sound)
                    };
                    if let Some(sound) = show {
                        let app = handle.clone();
                        let _ = handle.run_on_main_thread(move || {
                            if let Err(e) = show_reminder(&app, sound) {
                                log_error(&app, e.to_string());
                            }
                        });
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
                } else {
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("无法启动下班啦");
}
