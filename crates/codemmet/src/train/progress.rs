//! Cross-session progress: lifetime stats, completed lessons, and the
//! spaced-repetition schedule, persisted as JSON under the data directory.
//!
//! Loading is best-effort — a missing, unreadable, or corrupt file yields a
//! fresh [`Progress`] rather than an error, so a bad file never blocks a
//! session (and `--reset` can clear it).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::review::ReviewState;
use super::trainer::Stats;

/// The on-disk schema version, so future changes can migrate or discard.
const VERSION: u32 = 1;

/// Cumulative stats across every session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifetimeStats {
    /// Exercises shown.
    pub presented: u64,
    /// Exercises solved.
    pub solved: u64,
    /// Gradeable answers submitted.
    pub attempts: u64,
    /// Best streak ever.
    pub best_streak: u32,
    /// Number of sessions run.
    pub sessions: u64,
}

impl LifetimeStats {
    /// Fold a finished session's [`Stats`] into the lifetime totals.
    pub fn absorb(&mut self, session: &Stats) {
        self.presented += u64::from(session.presented);
        self.solved += u64::from(session.solved);
        self.attempts += u64::from(session.attempts);
        self.best_streak = self.best_streak.max(session.best_streak);
        self.sessions += 1;
    }
}

/// The full persisted progress document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// Schema version.
    pub version: u32,
    /// Lifetime tallies.
    pub lifetime: LifetimeStats,
    /// Completed lesson levels per language id.
    pub lessons: BTreeMap<String, BTreeSet<u8>>,
    /// Spaced-repetition schedule per language id.
    pub review: BTreeMap<String, ReviewState>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            version: VERSION,
            lifetime: LifetimeStats::default(),
            lessons: BTreeMap::new(),
            review: BTreeMap::new(),
        }
    }
}

impl Progress {
    /// The review schedule for `language` (empty if none yet).
    #[must_use]
    pub fn review_for(&self, language: &str) -> ReviewState {
        self.review.get(language).cloned().unwrap_or_default()
    }

    /// Store the review schedule for `language`.
    pub fn set_review(&mut self, language: &str, review: ReviewState) {
        self.review.insert(language.to_string(), review);
    }

    /// Mark `levels` completed for `language` (unions with any existing).
    pub fn mark_completed(&mut self, language: &str, levels: &BTreeSet<u8>) {
        self.lessons
            .entry(language.to_string())
            .or_default()
            .extend(levels.iter().copied());
    }
}

/// The default progress path, `<data-dir>/codemmet/trainer.json`.
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("codemmet").join("trainer.json"))
}

/// Load progress from `path`, or a fresh document if it is missing or unreadable.
#[must_use]
pub fn load(path: &Path) -> Progress {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Progress::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Save `progress` to `path`, creating the parent directory as needed.
pub fn save(path: &Path, progress: &Progress) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(progress).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

/// Delete the progress file if it exists.
pub fn reset(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::train::exercise::Concept;

    fn temp_path(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("codemmet-progress-test-{name}.json"));
        dir
    }

    #[test]
    fn absorb_accumulates_and_counts_sessions() {
        let mut lifetime = LifetimeStats::default();
        let session = Stats {
            presented: 4,
            solved: 3,
            attempts: 5,
            streak: 2,
            best_streak: 3,
        };
        lifetime.absorb(&session);
        lifetime.absorb(&session);
        assert_eq!(lifetime.presented, 8);
        assert_eq!(lifetime.solved, 6);
        assert_eq!(lifetime.best_streak, 3);
        assert_eq!(lifetime.sessions, 2);
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = temp_path("round-trip");
        let _ = reset(&path);

        let mut progress = Progress::default();
        progress.lifetime.absorb(&Stats {
            presented: 2,
            solved: 1,
            attempts: 2,
            streak: 1,
            best_streak: 1,
        });
        let mut review = ReviewState::default();
        review.record(&Concept::Trigger("fori".into()), true, 100);
        progress.set_review("rust", review);
        progress.mark_completed("rust", &BTreeSet::from([1, 2]));

        save(&path, &progress).expect("save");
        let loaded = load(&path);
        assert_eq!(loaded, progress);
        assert_eq!(loaded.lessons.get("rust"), Some(&BTreeSet::from([1, 2])));
        assert_eq!(loaded.review_for("rust").due_count(u64::MAX), 1);

        reset(&path).expect("reset");
        assert_eq!(load(&path), Progress::default());
    }

    #[test]
    fn loading_a_missing_file_is_a_fresh_document() {
        let path = temp_path("missing");
        let _ = reset(&path);
        assert_eq!(load(&path), Progress::default());
    }

    #[test]
    fn reset_is_ok_when_absent() {
        let path = temp_path("absent");
        let _ = reset(&path);
        assert!(reset(&path).is_ok());
    }
}
