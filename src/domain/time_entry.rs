use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::TaskId;

/// Unique identifier for time entries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeEntryId(pub Uuid);

impl TimeEntryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TimeEntryId {
    fn default() -> Self {
        Self::new()
    }
}

/// Time tracking entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: TimeEntryId,
    pub task_id: TaskId,
    pub description: Option<String>,

    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,

    /// Duration in minutes (calculated or manual)
    pub duration_minutes: Option<u32>,
}

impl TimeEntry {
    pub fn start(task_id: TaskId) -> Self {
        Self {
            id: TimeEntryId::new(),
            task_id,
            description: None,
            started_at: Utc::now(),
            ended_at: None,
            duration_minutes: None,
        }
    }

    pub fn stop(&mut self) {
        let end = Utc::now();
        self.ended_at = Some(end);
        self.duration_minutes = Some((end - self.started_at).num_minutes().max(0) as u32);
    }

    pub fn is_running(&self) -> bool {
        self.ended_at.is_none()
    }

    pub fn calculated_duration_minutes(&self) -> u32 {
        if let Some(duration) = self.duration_minutes {
            duration
        } else {
            let end = self.ended_at.unwrap_or_else(Utc::now);
            (end - self.started_at).num_minutes().max(0) as u32
        }
    }

    pub fn formatted_duration(&self) -> String {
        let minutes = self.calculated_duration_minutes();
        let hours = minutes / 60;
        let mins = minutes % 60;
        if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }
}
