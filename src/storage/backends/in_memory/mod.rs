//! Shared in-memory storage implementation.
//!
//! This module provides a trait and blanket implementations for storage backends
//! that use in-memory data structures (like JSON and YAML). By implementing the
//! [`InMemoryBackend`] trait, a backend automatically gets implementations for
//! all repository traits.

mod habit_repo;
mod project_repo;
mod tag_repo;
mod task_repo;
mod time_entry_repo;
mod work_log_repo;

use crate::storage::ExportData;

/// Trait for storage backends that use in-memory data structures.
///
/// Backends implementing this trait get automatic implementations of all
/// repository traits (TaskRepository, ProjectRepository, etc.) via blanket impls.
///
/// # Example
///
/// ```
/// use taskflow::storage::{ExportData, TaskRepository};
/// use taskflow::storage::backends::InMemoryBackend;
/// use taskflow::domain::Task;
///
/// struct MyBackend {
///     data: ExportData,
///     dirty: bool,
/// }
///
/// impl InMemoryBackend for MyBackend {
///     fn data(&self) -> &ExportData { &self.data }
///     fn data_mut(&mut self) -> &mut ExportData { &mut self.data }
///     fn mark_dirty(&mut self) { self.dirty = true; }
/// }
///
/// // MyBackend now automatically implements TaskRepository
/// let mut backend = MyBackend { data: ExportData::default(), dirty: false };
/// let task = Task::new("Test task");
/// backend.create_task(&task).unwrap();
/// assert_eq!(backend.list_tasks().unwrap().len(), 1);
/// ```
pub trait InMemoryBackend {
    /// Returns a reference to the in-memory data.
    fn data(&self) -> &ExportData;

    /// Returns a mutable reference to the in-memory data.
    fn data_mut(&mut self) -> &mut ExportData;

    /// Marks the data as modified and needing to be saved.
    fn mark_dirty(&mut self);
}
