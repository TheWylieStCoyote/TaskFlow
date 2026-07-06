//! Spaced repetition over concepts, using Leitner boxes.
//!
//! Each concept (a trigger or an operator) an exercise touches becomes a review
//! item. Answering it correctly promotes it to a higher box with a longer
//! interval; missing it drops it back to the first box. An item is *due* once
//! its `due` timestamp has passed. Time is injected as a Unix-seconds `now`, so
//! scheduling is fully deterministic in tests.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::exercise::Concept;

/// Review intervals per Leitner box, in days. Box 0 is due immediately.
const INTERVALS_DAYS: [u64; 6] = [0, 1, 3, 7, 14, 30];
/// Seconds in a day.
const SECS_PER_DAY: u64 = 86_400;

/// One tracked concept: which box it is in and when it next comes due.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewItem {
    /// The concept under review.
    pub concept: Concept,
    /// Leitner box index into [`INTERVALS_DAYS`].
    pub box_level: usize,
    /// When the item is next due, in Unix seconds.
    pub due: u64,
}

/// The spaced-repetition schedule for one language.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewState {
    items: BTreeMap<String, ReviewItem>,
}

impl ReviewState {
    /// Record an answer for `concept`, promoting on `correct` and demoting to
    /// the first box otherwise, then reschedule it from `now`.
    pub fn record(&mut self, concept: &Concept, correct: bool, now: u64) {
        let item = self
            .items
            .entry(concept.key())
            .or_insert_with(|| ReviewItem {
                concept: concept.clone(),
                box_level: 0,
                due: now,
            });
        item.box_level = if correct {
            (item.box_level + 1).min(INTERVALS_DAYS.len() - 1)
        } else {
            0
        };
        item.due = now + INTERVALS_DAYS[item.box_level] * SECS_PER_DAY;
    }

    /// The concepts due at `now`, soonest-due first.
    #[must_use]
    pub fn due(&self, now: u64) -> Vec<Concept> {
        let mut due: Vec<&ReviewItem> = self.items.values().filter(|i| i.due <= now).collect();
        due.sort_by_key(|i| (i.due, i.concept.key()));
        due.into_iter().map(|i| i.concept.clone()).collect()
    }

    /// How many items are due at `now`.
    #[must_use]
    pub fn due_count(&self, now: u64) -> usize {
        self.items.values().filter(|i| i.due <= now).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger(name: &str) -> Concept {
        Concept::Trigger(name.to_string())
    }

    #[test]
    fn a_correct_answer_promotes_and_schedules_ahead() {
        let mut state = ReviewState::default();
        let now = 1_000_000;
        state.record(&trigger("fori"), true, now);
        // Box 0 -> 1, so due one day out; not due now, due tomorrow.
        assert_eq!(state.due_count(now), 0);
        assert_eq!(state.due_count(now + SECS_PER_DAY), 1);
    }

    #[test]
    fn a_miss_drops_to_the_first_box_and_is_due_immediately() {
        let mut state = ReviewState::default();
        let now = 1_000_000;
        // Promote a few times, then miss.
        for _ in 0..3 {
            state.record(&trigger("pr"), true, now);
        }
        state.record(&trigger("pr"), false, now);
        // Box 0 interval is 0 days -> due right now.
        assert_eq!(state.due_count(now), 1);
        assert_eq!(state.due(now), vec![trigger("pr")]);
    }

    #[test]
    fn promotion_caps_at_the_last_box() {
        let mut state = ReviewState::default();
        let now = 0;
        for _ in 0..20 {
            state.record(&trigger("ret"), true, now);
        }
        // Never due sooner than the longest interval.
        let last = *INTERVALS_DAYS.last().expect("intervals are non-empty");
        assert_eq!(state.due_count(now + last * SECS_PER_DAY - 1), 0);
        assert_eq!(state.due_count(now + last * SECS_PER_DAY), 1);
    }

    #[test]
    fn due_is_sorted_soonest_first() {
        let mut state = ReviewState::default();
        state.record(&trigger("a"), false, 100); // due 100
        state.record(&trigger("b"), true, 50); // due 50 + 1 day
        let due = state.due(10_000_000);
        assert_eq!(due, vec![trigger("a"), trigger("b")]);
    }

    #[test]
    fn round_trips_through_json() {
        let mut state = ReviewState::default();
        state.record(&trigger("fori"), true, 42);
        state.record(&Concept::Operator('>'), false, 42);
        let json = serde_json::to_string(&state).expect("serialize");
        let back: ReviewState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, back);
        assert_eq!(back.due_count(u64::MAX), 2);
    }
}
