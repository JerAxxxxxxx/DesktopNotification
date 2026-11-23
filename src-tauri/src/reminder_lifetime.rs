use crate::model::Settings;
use serde::Serialize;
use std::time::{Duration, Instant};

const VISIBLE_FOR: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CardKind {
    Offwork,
    Preview,
    Sedentary,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderCard {
    pub id: u32,
    pub kind: CardKind,
    message: String,
    interval_minutes: u16,
}

#[derive(Default)]
pub struct ReminderLifetime {
    sequence: u32,
    active: Option<(Instant, ReminderCard)>,
}

impl ReminderLifetime {
    pub fn show(&mut self, now: Instant, settings: &Settings, kind: CardKind) -> Instant {
        let deadline = now + VISIBLE_FOR;
        self.sequence = self.sequence.wrapping_add(1);
        self.active = Some((
            deadline,
            ReminderCard {
                id: self.sequence,
                kind,
                message: match kind {
                    CardKind::Sedentary => format!(
                        "已经计时 {} 分钟了，起来走两步吧。",
                        settings.sedentary_minutes
                    ),
                    _ => settings.message.clone(),
                },
                interval_minutes: settings.interval_minutes,
            },
        ));
        deadline
    }

    pub fn card(&self) -> Option<&ReminderCard> {
        self.active.as_ref().map(|(_, card)| card)
    }

    pub fn action<'a>(&self, id: u32, action: &'a str) -> Option<&'a str> {
        let card = self.card().filter(|card| card.id == id)?;
        Some(if card.kind != CardKind::Offwork {
            "dismiss"
        } else {
            action
        })
    }

    pub fn clear(&mut self) {
        self.active = None;
    }

    pub fn should_hide(&self, deadline: Instant, now: Instant) -> bool {
        self.active
            .as_ref()
            .is_some_and(|(end, _)| *end == deadline && now >= deadline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_and_stale_clicks_cannot_pause_or_snooze_real_reminders() {
        let mut lifetime = ReminderLifetime::default();
        let now = Instant::now();
        let settings = crate::model::Settings::default();
        lifetime.show(now, &settings, CardKind::Preview);
        let preview = lifetime.card().unwrap().id;
        assert_eq!(lifetime.action(preview, "pause"), Some("dismiss"));
        assert_eq!(lifetime.action(preview, "snooze"), Some("dismiss"));
        lifetime.show(now + Duration::from_secs(1), &settings, CardKind::Offwork);
        let real = lifetime.card().unwrap().id;
        assert_eq!(lifetime.action(preview, "pause"), None);
        assert_eq!(lifetime.action(real, "pause"), Some("pause"));
        assert_eq!(lifetime.action(real, "snooze"), Some("snooze"));
        lifetime.clear();
        assert_eq!(lifetime.action(real, "snooze"), None);
    }

    #[test]
    fn sedentary_card_cannot_pause_offwork_reminders() {
        let mut lifetime = ReminderLifetime::default();
        lifetime.show(Instant::now(), &Settings::default(), CardKind::Sedentary);
        let id = lifetime.card().unwrap().id;
        assert_eq!(lifetime.action(id, "pause"), Some("dismiss"));
        assert_eq!(lifetime.action(id, "snooze"), Some("dismiss"));
    }

    #[test]
    fn closes_only_after_the_card_has_been_visible_for_fifteen_seconds() {
        let shown = Instant::now();
        let mut lifetime = ReminderLifetime::default();
        let deadline = lifetime.show(shown, &Settings::default(), CardKind::Offwork);
        assert!(!lifetime.should_hide(deadline, shown + Duration::from_millis(14_999)));
        assert!(lifetime.should_hide(deadline, shown + Duration::from_secs(15)));
    }

    #[test]
    fn old_timers_cannot_hide_a_new_or_manually_closed_card() {
        let shown = Instant::now();
        let mut lifetime = ReminderLifetime::default();
        let old = lifetime.show(shown, &Settings::default(), CardKind::Preview);
        let new = lifetime.show(
            shown + Duration::from_secs(10),
            &Settings::default(),
            CardKind::Offwork,
        );
        assert!(!lifetime.should_hide(old, new));
        assert!(lifetime.should_hide(new, new));
        lifetime.clear();
        assert!(!lifetime.should_hide(new, new));
    }
}
