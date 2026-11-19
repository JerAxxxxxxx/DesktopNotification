#[derive(Clone, Debug)]
pub struct Schedule {
    pub start: u16,
    pub end: u16,
    pub interval: u16,
    pub days: [bool; 7],
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            start: 1080,
            end: 1260,
            interval: 10,
            days: [true, true, true, true, true, false, false],
        }
    }
}

impl Schedule {
    pub fn validate(&self) -> Result<(), String> {
        if self.start >= self.end || self.end >= 1440 {
            return Err("停止时间必须晚于下班时间，且不跨午夜".into());
        }
        if !(1..=1440).contains(&self.interval) {
            return Err("提醒间隔应为 1～1440 分钟".into());
        }
        if !self.days.iter().any(|enabled| *enabled) {
            return Err("请至少选择一个提醒日".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Clock {
    pub date: i32,
    pub weekday: usize,
    pub second: u32,
    pub timestamp: i64,
    pub offset: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Scheduler {
    pub date: Option<i32>,
    pub paused: bool,
    pub next: Option<i64>,
    pub last_shown: Option<i64>,
    pub last_clock: Option<i64>,
    pub last_offset: Option<i32>,
}

impl Scheduler {
    pub fn tick(&mut self, config: &Schedule, now: Clock, can_show: bool) -> bool {
        if self.date != Some(now.date) {
            *self = Self {
                date: Some(now.date),
                ..Self::default()
            };
        }
        if self.last_offset.is_some_and(|offset| offset != now.offset)
            || self.last_clock.is_some_and(|clock| now.timestamp < clock)
        {
            self.next = None;
        }
        self.last_clock = Some(now.timestamp);
        self.last_offset = Some(now.offset);
        if self.paused || !config.days[now.weekday] || now.second >= u32::from(config.end) * 60 {
            self.next = None;
            return false;
        }
        let midnight = now.timestamp - i64::from(now.second);
        let start = midnight + i64::from(config.start) * 60;
        let end = midnight + i64::from(config.end) * 60;
        let interval = i64::from(config.interval) * 60;
        let due = self.next.unwrap_or_else(|| {
            self.last_shown
                .map_or(start, |last| (last + interval).max(start))
        });
        self.next = (due < end).then_some(due.max(start));
        if !can_show || self.next.is_none_or(|next| now.timestamp < next) {
            return false;
        }
        self.last_shown = Some(now.timestamp);
        let next = now.timestamp + interval;
        self.next = (next < end).then_some(next);
        true
    }
    pub fn pause(&mut self, now: Clock) {
        self.date = Some(now.date);
        self.paused = true;
        self.next = None;
    }
    pub fn resume(&mut self, config: &Schedule, now: Clock) {
        self.paused = false;
        self.next = None;
        self.last_shown = None;
        self.tick(config, now, false);
    }
    pub fn snooze(&mut self, config: &Schedule, now: Clock) {
        self.tick(config, now, false);
        if self.paused || !config.days[now.weekday] {
            return;
        }
        let midnight = now.timestamp - i64::from(now.second);
        let next = (now.timestamp + i64::from(config.interval) * 60)
            .max(midnight + i64::from(config.start) * 60);
        self.last_shown = Some(now.timestamp);
        self.next = (next < midnight + i64::from(config.end) * 60).then_some(next);
    }
    pub fn reconfigure(&mut self) {
        self.next = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(day: i32, weekday: usize, hour: u32, minute: u32) -> Clock {
        let second = hour * 3600 + minute * 60;
        Clock {
            date: day,
            weekday,
            second,
            timestamp: i64::from(day) * 86400 + i64::from(second),
            offset: 0,
        }
    }

    #[test]
    fn first_reminder_fires_at_start_and_not_twice() {
        let c = Schedule::default();
        let mut s = Scheduler::default();
        assert!(!s.tick(&c, at(1, 0, 17, 59), true));
        assert!(s.tick(&c, at(1, 0, 18, 0), true));
        assert!(!s.tick(&c, at(1, 0, 18, 0), true));
    }
    #[test]
    fn repeats_only_after_the_interval() {
        let c = Schedule::default();
        let mut s = Scheduler::default();
        assert!(s.tick(&c, at(1, 0, 18, 0), true));
        assert!(!s.tick(&c, at(1, 0, 18, 9), true));
        assert!(s.tick(&c, at(1, 0, 18, 10), true));
    }
    #[test]
    fn missed_reminders_merge_into_one_after_resume() {
        let c = Schedule::default();
        let mut s = Scheduler::default();
        s.tick(&c, at(1, 0, 18, 0), true);
        assert!(!s.tick(&c, at(1, 0, 18, 30), false));
        assert!(s.tick(&c, at(1, 0, 18, 35), true));
        assert_eq!(s.next, Some(153900)); // day 1, 18:45
        assert!(!s.tick(&c, at(1, 0, 18, 35), true));
    }
    #[test]
    fn pause_survives_same_day_but_resets_next_day() {
        let c = Schedule::default();
        let mut s = Scheduler::default();
        s.pause(at(1, 0, 17, 0));
        assert!(!s.tick(&c, at(1, 0, 18, 0), true));
        assert!(s.tick(&c, at(2, 1, 18, 0), true));
    }
    #[test]
    fn no_reminder_on_weekends_or_at_cutoff() {
        let c = Schedule::default();
        let mut s = Scheduler::default();
        assert!(!s.tick(&c, at(1, 5, 18, 0), true));
        assert_eq!(s.next, None);
        assert!(!s.tick(&c, at(2, 1, 21, 0), true));
        assert_eq!(s.next, None);
    }
    #[test]
    fn snooze_moves_next_reminder_and_respects_cutoff() {
        let c = Schedule::default();
        let mut s = Scheduler::default();
        s.tick(&c, at(1, 0, 18, 0), true);
        s.snooze(&c, at(1, 0, 18, 5));
        assert!(!s.tick(&c, at(1, 0, 18, 10), true));
        assert!(s.tick(&c, at(1, 0, 18, 15), true));
        s.snooze(&c, at(1, 0, 20, 55));
        assert_eq!(s.next, None);
    }
    #[test]
    fn resume_clears_pause_and_reminds_when_due() {
        let c = Schedule::default();
        let mut s = Scheduler::default();
        s.pause(at(1, 0, 18, 0));
        s.resume(&c, at(1, 0, 18, 5));
        assert!(s.tick(&c, at(1, 0, 18, 5), true));
    }
    #[test]
    fn moving_clock_back_does_not_repeat_previous_reminder() {
        let c = Schedule::default();
        let mut s = Scheduler::default();
        s.tick(&c, at(1, 0, 18, 10), true);
        assert!(!s.tick(&c, at(1, 0, 18, 0), true));
        assert!(!s.tick(&c, at(1, 0, 18, 10), true));
        assert!(s.tick(&c, at(1, 0, 18, 20), true));
    }
    #[test]
    fn reconfigure_applies_new_start_time_without_repeating() {
        let mut c = Schedule::default();
        let mut s = Scheduler::default();
        s.tick(&c, at(1, 0, 17, 0), true);
        c.start = 1020;
        s.reconfigure();
        assert!(s.tick(&c, at(1, 0, 17, 0), true));
        s.reconfigure();
        assert!(!s.tick(&c, at(1, 0, 17, 0), true));
    }
    #[test]
    fn invalid_intervals_times_and_empty_days_are_rejected() {
        let mut c = Schedule {
            interval: 0,
            ..Schedule::default()
        };
        assert!(c.validate().is_err());
        c.interval = 1441;
        assert!(c.validate().is_err());
        c.interval = 10;
        c.end = c.start;
        assert!(c.validate().is_err());
        c.end = 1440;
        assert!(c.validate().is_err());
        c.end = 1260;
        c.days = [false; 7];
        assert!(c.validate().is_err());
    }
}
