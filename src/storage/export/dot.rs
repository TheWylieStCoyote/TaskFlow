//! DOT (Graphviz) export functionality.

use std::collections::HashMap;
use std::io::Write;

use crate::domain::{Task, TaskId, TaskStatus};

/// Exports task chains/dependencies to DOT (Graphviz) format.
///
/// This creates a directed graph showing:
/// - Task nodes with color-coded status (green=done, yellow=in progress, red=blocked)
/// - Chain edges (blue) from `next_task_id` relationships
/// - Dependency edges (red dashed) from `dependencies` relationships
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_dot<W: Write>(
    tasks: &HashMap<TaskId, Task>,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "digraph TaskChains {{")?;
    writeln!(writer, "    rankdir=LR;")?;
    writeln!(
        writer,
        "    node [shape=box, style=filled, fontname=\"Arial\"];"
    )?;
    writeln!(writer)?;

    // Write all nodes
    writeln!(writer, "    // Nodes")?;
    for task in tasks.values() {
        let node_id = format!("task_{}", task.id.0.to_string().replace('-', "_"));
        let label = escape_dot(&task.title);
        let fill_color = match task.status {
            TaskStatus::Done => "\"#90EE90\"",       // Light green
            TaskStatus::Cancelled => "\"#D3D3D3\"",  // Light gray
            TaskStatus::InProgress => "\"#FFD700\"", // Gold
            TaskStatus::Blocked => "\"#FFB6C1\"",    // Light pink
            TaskStatus::Todo => "\"#FFFFFF\"",       // White
        };
        writeln!(
            writer,
            "    {node_id} [label=\"{label}\" fillcolor={fill_color}];"
        )?;
    }

    writeln!(writer)?;

    // Write chain edges (next_task_id)
    writeln!(writer, "    // Chain edges (next_task_id)")?;
    for task in tasks.values() {
        if let Some(ref next_id) = task.next_task_id {
            if tasks.contains_key(next_id) {
                let from_id = format!("task_{}", task.id.0.to_string().replace('-', "_"));
                let to_id = format!("task_{}", next_id.0.to_string().replace('-', "_"));
                writeln!(
                    writer,
                    "    {from_id} -> {to_id} [color=\"blue\" label=\"chain\"];"
                )?;
            }
        }
    }

    writeln!(writer)?;

    // Write dependency edges
    writeln!(writer, "    // Dependency edges (blocks)")?;
    for task in tasks.values() {
        for dep_id in &task.dependencies {
            if tasks.contains_key(dep_id) {
                let from_id = format!("task_{}", dep_id.0.to_string().replace('-', "_"));
                let to_id = format!("task_{}", task.id.0.to_string().replace('-', "_"));
                writeln!(
                    writer,
                    "    {from_id} -> {to_id} [color=\"red\" style=\"dashed\" label=\"blocks\"];"
                )?;
            }
        }
    }

    writeln!(writer, "}}")?;
    Ok(())
}

/// Escape special characters for DOT format
fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_task(title: &str) -> Task {
        Task::new(title)
    }

    #[test]
    fn test_escape_dot() {
        assert_eq!(escape_dot("hello"), "hello");
        assert_eq!(escape_dot("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_dot("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_dot("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_export_dot_basic() {
        let mut tasks = HashMap::new();
        let task = create_test_task("Test Task");
        tasks.insert(task.id, task);

        let mut buffer = Vec::new();
        export_to_dot(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.starts_with("digraph TaskChains {"));
        assert!(result.contains("rankdir=LR"));
        assert!(result.contains("label=\"Test Task\""));
        assert!(result.ends_with("}\n"));
    }

    #[test]
    fn test_export_dot_with_status_colors() {
        let mut tasks = HashMap::new();

        let mut done_task = create_test_task("Done");
        done_task.status = TaskStatus::Done;
        tasks.insert(done_task.id, done_task);

        let mut in_progress_task = create_test_task("In Progress");
        in_progress_task.status = TaskStatus::InProgress;
        tasks.insert(in_progress_task.id, in_progress_task);

        let mut buffer = Vec::new();
        export_to_dot(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("#90EE90")); // Green for done
        assert!(result.contains("#FFD700")); // Gold for in progress
    }

    #[test]
    fn test_export_dot_with_chain() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("First");
        let task1_id = task1.id;
        tasks.insert(task1.id, task1);

        let mut task2 = create_test_task("Second");
        task2.next_task_id = Some(task1_id);
        tasks.insert(task2.id, task2);

        let mut buffer = Vec::new();
        export_to_dot(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("color=\"blue\""));
        assert!(result.contains("label=\"chain\""));
    }

    #[test]
    fn test_export_dot_with_dependency() {
        let mut tasks = HashMap::new();

        let blocker = create_test_task("Blocker");
        let blocker_id = blocker.id;
        tasks.insert(blocker.id, blocker);

        let mut blocked = create_test_task("Blocked");
        blocked.dependencies.push(blocker_id);
        tasks.insert(blocked.id, blocked);

        let mut buffer = Vec::new();
        export_to_dot(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains("color=\"red\""));
        assert!(result.contains("style=\"dashed\""));
        assert!(result.contains("label=\"blocks\""));
    }

    #[test]
    fn test_export_dot_empty() {
        let tasks: HashMap<TaskId, Task> = HashMap::new();
        let mut buffer = Vec::new();
        export_to_dot(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.starts_with("digraph TaskChains {"));
        assert!(result.ends_with("}\n"));
    }
}
