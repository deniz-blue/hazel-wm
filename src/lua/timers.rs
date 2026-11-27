#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueTimer {
    pub id: u64,
    pub repeats: bool,
}

#[derive(Debug, Default)]
pub struct Timers {
    next_id: u64,
    entries: Vec<Entry>,
}

#[derive(Debug)]
struct Entry {
    id: u64,
    due_ms: u64,
    interval_ms: Option<u64>,
}

impl Timers {
    pub fn add(&mut self, now_ms: u64, delay_ms: u64, repeats: bool) -> u64 {
        self.next_id += 1;
        self.entries.push(Entry {
            id: self.next_id,
            due_ms: now_ms.saturating_add(delay_ms),
            interval_ms: repeats.then_some(delay_ms.max(1)),
        });
        self.next_id
    }

    pub fn cancel(&mut self, id: u64) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != before
    }

    pub fn take_due(&mut self, now_ms: u64) -> Vec<DueTimer> {
        let mut due: Vec<&mut Entry> = self
            .entries
            .iter_mut()
            .filter(|entry| entry.due_ms <= now_ms)
            .collect();
        due.sort_by_key(|entry| (entry.due_ms, entry.id));
        let fired: Vec<DueTimer> = due
            .into_iter()
            .map(|entry| {
                if let Some(interval) = entry.interval_ms {
                    entry.due_ms = now_ms.saturating_add(interval);
                }
                DueTimer {
                    id: entry.id,
                    repeats: entry.interval_ms.is_some(),
                }
            })
            .collect();
        self.entries
            .retain(|entry| entry.interval_ms.is_some() || !fired.iter().any(|f| f.id == entry.id));
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_delay_too_long_to_add_never_fires_and_does_not_overflow() {
        let mut timers = Timers::default();
        timers.add(1000, u64::MAX, false);
        assert!(timers.take_due(u64::MAX - 1).is_empty());
    }

    #[test]
    fn a_timer_fires_once_when_due() {
        let mut timers = Timers::default();
        let id = timers.add(100, 50, false);
        assert!(timers.take_due(149).is_empty());
        assert_eq!(timers.take_due(150), [DueTimer { id, repeats: false }]);
        assert!(timers.take_due(1000).is_empty());
    }

    #[test]
    fn a_repeating_timer_is_rescheduled_from_now() {
        let mut timers = Timers::default();
        let id = timers.add(0, 100, true);
        assert_eq!(timers.take_due(100).len(), 1);
        assert!(timers.take_due(150).is_empty());
        assert_eq!(timers.take_due(450), [DueTimer { id, repeats: true }]);
    }

    #[test]
    fn cancelled_timers_never_fire() {
        let mut timers = Timers::default();
        let id = timers.add(0, 10, true);
        assert!(timers.cancel(id));
        assert!(!timers.cancel(id));
        assert!(timers.take_due(100).is_empty());
    }

    #[test]
    fn due_timers_fire_in_order_of_their_deadline() {
        let mut timers = Timers::default();
        let late = timers.add(0, 30, false);
        let early = timers.add(0, 10, false);
        let ids: Vec<u64> = timers.take_due(50).iter().map(|t| t.id).collect();
        assert_eq!(ids, [early, late]);
    }
}
