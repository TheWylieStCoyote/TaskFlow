//! CLI command handlers.

mod add;
mod done;
mod git_todos;
mod list;
mod next;
pub mod pipe;
mod stats;
mod today;

pub use add::quick_add_task;
pub use done::mark_task_done;
pub use git_todos::extract_git_todos;
pub use list::list_tasks;
pub use next::next_task;
pub use pipe::run_pipe;
pub use stats::show_stats;
pub use today::today_tasks;
