use crate::scheduler::{Clock, Schedule};

#[derive(Default)]
pub struct SedentaryTimer {
    last_check: Option<Clock>,
    minutes: u16,
    due_at: i64,
}

impl SedentaryTimer {
    pub fn tick(
        &mut self,
        now: Clock,
        schedule: &Schedule,
        minutes: u16,
        desktop_available: impl FnOnce() -> bool,
        deliver: impl FnOnce() -> bool,
    ) {
        if minutes == 0
            || !schedule.days[now.weekday]
            || now.second >= u32::from(schedule.start) * 60
            || !desktop_available()
        {
            self.last_check = None;
            return;
        }
        let interval = i64::from(minutes) * 60;
        // The host polls every two seconds. A long gap can include system sleep.
        let continuous = self.last_check.is_some_and(|last| {
            last.date == now.date
                && last.offset == now.offset
                && (0..=10).contains(&(now.timestamp - last.timestamp))
        });
        let new_session = !continuous || minutes != self.minutes;
        if new_session || (now.timestamp >= self.due_at && deliver()) {
            self.due_at = now.timestamp + interval;
        }
        self.last_check = Some(now);
        self.minutes = minutes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{Clock, Schedule};

    fn at(second: u32) -> Clock {
        Clock {
            date: 10,
            weekday: 0,
            second,
            timestamp: i64::from(second),
            offset: 0,
        }
    }

    #[test]
    fn repeats_only_after_a_full_interval_and_retries_failed_delivery() {
        let mut timer = SedentaryTimer::default();
        let mut shown = Vec::new();
        for second in (0..=3604).step_by(2) {
            timer.tick(
                at(second),
                &Schedule::default(),
                30,
                || true,
                || {
                    shown.push(second);
                    second != 1800
                },
            );
        }
        assert_eq!(shown, [1800, 1802, 3602]);
    }

    #[test]
    fn lock_sleep_and_interval_changes_start_a_fresh_interval() {
        let mut timer = SedentaryTimer::default();
        let mut shown = Vec::new();
        for second in (0..=12000)
            .step_by(2)
            .filter(|s| !(4000..=6400).contains(s))
        {
            timer.tick(
                at(second),
                &Schedule::default(),
                if second < 7900 { 30 } else { 60 },
                || second != 1500,
                || {
                    shown.push(second);
                    true
                },
            );
        }
        assert_eq!(shown, [3302, 11500]);
    }

    #[test]
    fn disabled_days_and_offwork_hours_do_not_even_probe_the_desktop() {
        let mut timer = SedentaryTimer::default();
        for (now, minutes) in [
            (at(100), 0),
            (
                Clock {
                    weekday: 6,
                    ..at(100)
                },
                30,
            ),
            (at(18 * 3600), 30),
        ] {
            timer.tick(
                now,
                &Schedule::default(),
                minutes,
                || panic!("outside reminder hours"),
                || panic!("unexpected reminder"),
            );
        }
    }
}
