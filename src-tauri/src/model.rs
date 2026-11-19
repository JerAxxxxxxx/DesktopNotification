use crate::scheduler::{Clock, Schedule, Scheduler};
use chrono::{Datelike, Local, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub start_time: String,
    pub end_time: String,
    pub interval_minutes: u16,
    pub days: [bool; 7],
    pub message: String,
    pub sound: bool,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_time: "18:00".into(),
            end_time: "21:00".into(),
            interval_minutes: 10,
            days: Schedule::default().days,
            message: "今天也辛苦了，收拾一下，准备下班吧。".into(),
            sound: true,
            autostart: false,
        }
    }
}

impl Settings {
    pub fn schedule(&self) -> Result<Schedule, String> {
        fn minutes(value: &str) -> Result<u16, String> {
            let invalid = || "请输入有效的时间（HH:MM）".to_string();
            if value.len() != 5
                || value.as_bytes()[2] != b':'
                || !value
                    .bytes()
                    .enumerate()
                    .all(|(index, byte)| index == 2 || byte.is_ascii_digit())
            {
                return Err(invalid());
            }
            let hour: u16 = value
                .get(..2)
                .ok_or_else(invalid)?
                .parse()
                .map_err(|_| invalid())?;
            let minute: u16 = value
                .get(3..)
                .ok_or_else(invalid)?
                .parse()
                .map_err(|_| invalid())?;
            if hour >= 24 || minute >= 60 {
                return Err(invalid());
            }
            Ok(hour * 60 + minute)
        }
        if self.message.trim().is_empty() || self.message.chars().count() > 200 {
            return Err("提醒文案需要填写 1～200 个字符".into());
        }
        let schedule = Schedule {
            start: minutes(&self.start_time)?,
            end: minutes(&self.end_time)?,
            interval: self.interval_minutes,
            days: self.days,
        };
        schedule.validate()?;
        Ok(schedule)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Daily {
    pub date: Option<i32>,
    pub paused: bool,
    pub next: Option<i64>,
    pub last_shown: Option<i64>,
    pub last_clock: Option<i64>,
    pub last_offset: Option<i32>,
}

impl From<&Scheduler> for Daily {
    fn from(s: &Scheduler) -> Self {
        Self {
            date: s.date,
            paused: s.paused,
            next: s.next,
            last_shown: s.last_shown,
            last_clock: s.last_clock,
            last_offset: s.last_offset,
        }
    }
}

impl Daily {
    pub fn restore(&self) -> Scheduler {
        Scheduler {
            date: self.date,
            paused: self.paused,
            next: self.next,
            last_shown: self.last_shown,
            last_clock: self.last_clock,
            last_offset: self.last_offset,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub settings: Settings,
    pub daily: Daily,
    #[serde(rename = "countdownVisible")]
    pub countdown_visible: bool,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            daily: Daily::default(),
            countdown_visible: true,
        }
    }
}

pub fn clock() -> Clock {
    let now = Local::now();
    Clock {
        date: now.date_naive().num_days_from_ce(),
        weekday: now.weekday().num_days_from_monday() as usize,
        second: now.num_seconds_from_midnight(),
        timestamp: now.timestamp(),
        offset: now.offset().local_minus_utc(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub countdown_visible: bool,
    pub paused: bool,
    pub phase: String,
    pub next_at: Option<i64>,
    pub warning: Option<String>,
    pub config_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_is_visible_for_old_settings_and_can_stay_hidden() {
        let old: Saved = serde_json::from_str(r#"{"settings":{"startTime":"17:30"}}"#).unwrap();
        assert!(old.countdown_visible);
        assert_eq!(old.settings.start_time, "17:30");

        let hidden = Saved {
            countdown_visible: false,
            ..old
        };
        let round_trip: Saved =
            serde_json::from_str(&serde_json::to_string(&hidden).unwrap()).unwrap();
        assert!(!round_trip.countdown_visible);
    }

    #[test]
    fn time_parser_rejects_signed_and_non_clock_values() {
        for invalid in ["+1:00", "18:60", "24:00", "8:00", "今天下班", "18:0a"] {
            let settings = Settings {
                start_time: invalid.into(),
                ..Settings::default()
            };
            assert!(
                settings.schedule().is_err(),
                "accepted invalid time: {invalid}"
            );
        }
    }

    #[test]
    fn serialized_daily_pause_survives_restart_and_expires_next_day() {
        let saved = Saved {
            daily: Daily {
                date: Some(1),
                paused: true,
                ..Daily::default()
            },
            ..Saved::default()
        };
        let data = serde_json::to_string(&saved).unwrap();
        let loaded: Saved = serde_json::from_str(&data).unwrap();
        let config = loaded.settings.schedule().unwrap();
        let mut scheduler = loaded.daily.restore();
        let now = Clock {
            date: 1,
            weekday: 0,
            second: 64800,
            timestamp: 151200,
            offset: 0,
        };
        assert!(!scheduler.tick(&config, now, true));
        assert!(scheduler.tick(
            &config,
            Clock {
                date: 2,
                weekday: 1,
                timestamp: 237600,
                ..now
            },
            true
        ));
    }

    #[test]
    fn settings_reject_blank_and_overlong_reminder_text() {
        let mut settings = Settings {
            message: "  ".into(),
            ..Settings::default()
        };
        assert!(settings.schedule().is_err());
        settings.message = "下".repeat(201);
        assert!(settings.schedule().is_err());
        settings.message = "下".repeat(200);
        assert!(settings.schedule().is_ok());
    }
}
