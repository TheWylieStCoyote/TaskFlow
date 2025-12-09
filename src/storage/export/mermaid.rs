//! Mermaid diagram export functionality.

use std::collections::HashMap;
use std::io::Write;

use crate::domain::{Task, TaskId, TaskStatus};

/// Exports task chains/dependencies to Mermaid format.
///
/// This creates a flowchart showing:
/// - Task nodes with styled classes for status
/// - Chain edges (solid arrows) from `next_task_id` relationships
/// - Dependency edges (dotted arrows with "blocks" label) from `dependencies` relationships
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_mermaid<W: Write>(
    tasks: &HashMap<TaskId, Task>,
    writer: &mut W,
) -> std::io::Result<()> {
    writeln!(writer, "```mermaid")?;
    writeln!(writer, "flowchart LR")?;
    writeln!(writer)?;

    // Define style classes
    writeln!(writer, "    %% Style definitions")?;
    writeln!(writer, "    classDef done fill:#90EE90,stroke:#228B22")?;
    writeln!(writer, "    classDef cancelled fill:#D3D3D3,stroke:#696969")?;
    writeln!(
        writer,
        "    classDef inprogress fill:#FFD700,stroke:#DAA520"
    )?;
    writeln!(writer, "    classDef blocked fill:#FFB6C1,stroke:#DC143C")?;
    writeln!(writer, "    classDef todo fill:#FFFFFF,stroke:#333333")?;
    writeln!(writer)?;

    // Write nodes with shorter IDs for readability
    let id_map: HashMap<TaskId, String> = tasks
        .keys()
        .enumerate()
        .map(|(i, id)| (*id, format!("T{i}")))
        .collect();

    writeln!(writer, "    %% Nodes")?;
    for (task_id, task) in tasks {
        // id_map is built from the same keys, so this lookup always succeeds
        let Some(short_id) = id_map.get(task_id) else {
            continue;
        };
        let label = escape_mermaid(&task.title);
        let class = match task.status {
            TaskStatus::Done => "done",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::InProgress => "inprogress",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Todo => "todo",
        };
        writeln!(writer, "    {short_id}[\"{label}\"]:::{class}")?;
    }

    writeln!(writer)?;

    // Write chain edges (next_task_id)
    writeln!(writer, "    %% Chain edges (next task in sequence)")?;
    for task in tasks.values() {
        if let Some(ref next_id) = task.next_task_id {
            if let (Some(from), Some(to)) = (id_map.get(&task.id), id_map.get(next_id)) {
                writeln!(writer, "    {from} --> {to}")?;
            }
        }
    }

    writeln!(writer)?;

    // Write dependency edges
    writeln!(writer, "    %% Dependency edges (blocks)")?;
    for task in tasks.values() {
        for dep_id in &task.dependencies {
            if let (Some(from), Some(to)) = (id_map.get(dep_id), id_map.get(&task.id)) {
                writeln!(writer, "    {from} -.->|blocks| {to}")?;
            }
        }
    }

    writeln!(writer, "```")?;
    Ok(())
}

/// Escape special characters for Mermaid format
fn escape_mermaid(s: &str) -> String {
    s.replace('"', "'")
        .replace('[', "(")
        .replace(']', ")")
        .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_task(title: &str) -> Task {
        Task::new(title)
    }

    #[test]
    fn test_escape_mermaid() {
        assert_eq!(escape_mermaid("hello"), "hello");
        assert_eq!(escape_mermaid("say \"hi\""), "say 'hi'");
        assert_eq!(escape_mermaid("[task]"), "(task)");
        assert_eq!(escape_mermaid("line1\nline2"), "line1 line2");
    }

    #[test]
    fn test_export_mermaid_basic() {
        let mut tasks = HashMap::new();
        let task = create_test_task("Test Task");
        tasks.insert(task.id, task);

        let mut buffer = Vec::new();
        export_to_mermaid(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.starts_with("```mermaid"));
        assert!(result.contains("flowchart LR"));
        assert!(result.contains("Test Task"));
        assert!(result.contains("classDef"));
        assert!(result.ends_with("```\n"));
    }

    #[test]
    fn test_export_mermaid_with_status_classes() {
        let mut tasks = HashMap::new();

        let mut done_task = create_test_task("Done");
        done_task.status = TaskStatus::Done;
        tasks.insert(done_task.id, done_task);

        let mut in_progress_task = create_test_task("In Progress");
        in_progress_task.status = TaskStatus::InProgress;
        tasks.insert(in_progress_task.id, in_progress_task);

        let mut buffer = Vec::new();
        export_to_mermaid(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.contains(":::done"));
        assert!(result.contains(":::inprogress"));
    }

    #[test]
    fn test_export_mermaid_with_chain() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("First");
        let task1_id = task1.id;
        tasks.insert(task1.id, task1);

        let mut task2 = create_test_task("Second");
        task2.next_task_id = Some(task1_id);
        tasks.insert(task2.id, task2);

        let mut buffer = Vec::new();
        export_to_mermaid(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        // Should have solid arrow for chain
        assert!(result.contains("-->"));
    }

    #[test]
    fn test_export_mermaid_with_dependency() {
        let mut tasks = HashMap::new();

        let blocker = create_test_task("Blocker");
        let blocker_id = blocker.id;
        tasks.insert(blocker.id, blocker);

        let mut blocked = create_test_task("Blocked");
        blocked.dependencies.push(blocker_id);
        tasks.insert(blocked.id, blocked);

        let mut buffer = Vec::new();
        export_to_mermaid(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        // Should have dotted arrow with label for dependency
        assert!(result.contains("-.->|blocks|"));
    }

    #[test]
    fn test_export_mermaid_empty() {
        let tasks: HashMap<TaskId, Task> = HashMap::new();
        let mut buffer = Vec::new();
        export_to_mermaid(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        assert!(result.starts_with("```mermaid"));
        assert!(result.ends_with("```\n"));
    }

    #[test]
    fn test_export_mermaid_short_ids() {
        let mut tasks = HashMap::new();

        for i in 0..3 {
            let task = create_test_task(&format!("Task {i}"));
            tasks.insert(task.id, task);
        }

        let mut buffer = Vec::new();
        export_to_mermaid(&tasks, &mut buffer).unwrap();
        let result = String::from_utf8(buffer).unwrap();

        // Should use short IDs like T0, T1, T2
        assert!(result.contains("T0["));
        assert!(result.contains("T1["));
        assert!(result.contains("T2["));
    }
}
