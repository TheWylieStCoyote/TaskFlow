//! CLI command handlers.

mod add;
mod done;
mod list;

pub use add::quick_add_task;
pub use done::mark_task_done;
pub use list::list_tasks;
