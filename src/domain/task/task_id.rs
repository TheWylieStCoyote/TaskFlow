//! Task identifier type.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for tasks.
///
/// Each task has a UUID-based identifier that remains stable across
/// serialization and storage operations.
///
/// # Examples
///
/// ```
/// use taskflow::domain::TaskId;
///
/// let id1 = TaskId::new();
/// let id2 = TaskId::new();
/// assert_ne!(id1, id2); // Each ID is unique
///
/// // Can be displayed as a string
/// println!("Task ID: {}", id1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    /// Creates a new unique task identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
