//! Undo action types and their implementations.

use crate::domain::{Project, Task, TimeEntry, WorkLogEntry};

use super::{action_desc, inverse_clone, inverse_swap};

/// Represents an action that can be undone/redone
#[derive(Debug, Clone)]
pub enum UndoAction {
    /// Task was created - undo by deleting it
    TaskCreated(Box<Task>),
    /// Task was deleted - undo by restoring it (includes associated time entries)
    TaskDeleted {
        task: Box<Task>,
        time_entries: Vec<TimeEntry>,
    },
    /// Task was modified - undo by restoring previous state
    TaskModified { before: Box<Task>, after: Box<Task> },
    /// Project was created - undo by deleting it
    ProjectCreated(Box<Project>),
    /// Project was deleted - undo by restoring it
    ProjectDeleted(Box<Project>),
    /// Project was modified - undo by restoring previous state
    ProjectModified {
        before: Box<Project>,
        after: Box<Project>,
    },
    /// Time entry was created (started) - undo by deleting it
    TimeEntryStarted(Box<TimeEntry>),
    /// Time entry was stopped - undo by restoring running state
    TimeEntryStopped {
        before: Box<TimeEntry>,
        after: Box<TimeEntry>,
    },
    /// Time entry was deleted - undo by restoring it
    TimeEntryDeleted(Box<TimeEntry>),
    /// Time entry was modified - undo by restoring previous state
    TimeEntryModified {
        before: Box<TimeEntry>,
        after: Box<TimeEntry>,
    },
    /// Timer was switched from one task to another - undo both in one operation
    TimerSwitched {
        stopped_entry_before: Box<TimeEntry>,
        stopped_entry_after: Box<TimeEntry>,
        started_entry: Box<TimeEntry>,
    },
    /// Work log entry was created - undo by deleting it
    WorkLogCreated(Box<WorkLogEntry>),
    /// Work log entry was deleted - undo by restoring it
    WorkLogDeleted(Box<WorkLogEntry>),
    /// Work log entry was modified - undo by restoring previous state
    WorkLogModified {
        before: Box<WorkLogEntry>,
        after: Box<WorkLogEntry>,
    },
}

impl UndoAction {
    /// Get a human-readable description of the action
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            // Task actions
            Self::TaskCreated(task) => action_desc!(create "task", &task.title),
            Self::TaskDeleted { task, .. } => action_desc!(delete "task", &task.title),
            Self::TaskModified { before, .. } => action_desc!(modify "task", &before.title),

            // Project actions
            Self::ProjectCreated(project) => action_desc!(create "project", &project.name),
            Self::ProjectDeleted(project) => action_desc!(delete "project", &project.name),
            Self::ProjectModified { before, .. } => action_desc!(modify "project", &before.name),

            // Time entry actions (no entity name - actions are self-explanatory)
            Self::TimeEntryStarted(_) => "Start time tracking".to_string(),
            Self::TimeEntryStopped { .. } => "Stop time tracking".to_string(),
            Self::TimeEntryDeleted(_) => "Delete time entry".to_string(),
            Self::TimeEntryModified { .. } => "Modify time entry".to_string(),
            Self::TimerSwitched { .. } => "Switch timer".to_string(),

            // Work log actions (use add/edit verbs for consistency with UI)
            Self::WorkLogCreated(entry) => action_desc!(add "work log", entry.summary()),
            Self::WorkLogDeleted(entry) => action_desc!(delete "work log", entry.summary()),
            Self::WorkLogModified { before, .. } => action_desc!(edit "work log", before.summary()),
        }
    }

    /// Get the inverse action for redo
    #[must_use]
    pub fn inverse(&self) -> Self {
        match self {
            // Self-inverse actions: clone as-is (create/delete pairs)
            Self::TaskCreated(task) => inverse_clone!(task, TaskCreated),
            Self::TaskDeleted { task, time_entries } => {
                inverse_clone!(self, TaskDeleted { task, time_entries })
            }
            Self::ProjectCreated(project) => inverse_clone!(project, ProjectCreated),
            Self::ProjectDeleted(project) => inverse_clone!(project, ProjectDeleted),
            Self::TimeEntryStarted(entry) => inverse_clone!(entry, TimeEntryStarted),
            Self::TimeEntryDeleted(entry) => inverse_clone!(entry, TimeEntryDeleted),
            Self::WorkLogCreated(entry) => inverse_clone!(entry, WorkLogCreated),
            Self::WorkLogDeleted(entry) => inverse_clone!(entry, WorkLogDeleted),

            // Swap-inverse actions: swap before/after (modify actions)
            Self::TaskModified { before, after } => inverse_swap!(TaskModified, before, after),
            Self::ProjectModified { before, after } => {
                inverse_swap!(ProjectModified, before, after)
            }
            Self::TimeEntryStopped { before, after } => {
                inverse_swap!(TimeEntryStopped, before, after)
            }
            Self::TimeEntryModified { before, after } => {
                inverse_swap!(TimeEntryModified, before, after)
            }
            Self::WorkLogModified { before, after } => {
                inverse_swap!(WorkLogModified, before, after)
            }

            // Special case: TimerSwitched has 3 fields, clone all as-is
            Self::TimerSwitched {
                stopped_entry_before,
                stopped_entry_after,
                started_entry,
            } => Self::TimerSwitched {
                stopped_entry_before: stopped_entry_before.clone(),
                stopped_entry_after: stopped_entry_after.clone(),
                started_entry: started_entry.clone(),
            },
        }
    }
}
