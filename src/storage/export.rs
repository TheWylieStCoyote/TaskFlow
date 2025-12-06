use std::collections::HashMap;
use std::io::Write;

use crate::domain::{Priority, Task, TaskId, TaskStatus};

/// Export format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Ics,
    Dot,
    Mermaid,
}

impl ExportFormat {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "ics" | "ical" | "icalendar" => Some(Self::Ics),
            "dot" | "graphviz" => Some(Self::Dot),
            "mermaid" | "md" => Some(Self::Mermaid),
            _ => None,
        }
    }

    #[must_use]
    pub const fn file_extension(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Ics => "ics",
            Self::Dot => "dot",
            Self::Mermaid => "md",
        }
    }
}

/// Exports tasks to CSV format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_csv<W: Write>(tasks: &[Task], writer: &mut W) -> std::io::Result<()> {
    // Write header
    writeln!(
        writer,
        "ID,Title,Status,Priority,Due Date,Tags,Project ID,Description,Created,Completed"
    )?;

    for task in tasks {
        let id = task.id.0.to_string();
        let title = escape_csv(&task.title);
        let status = task.status.as_str();
        let priority = task.priority.as_str();
        let due_date = task.due_date.map(|d| d.to_string()).unwrap_or_default();
        let tags = task.tags.join(";");
        let project_id = task
            .project_id
            .as_ref()
            .map(|p| p.0.to_string())
            .unwrap_or_default();
        let description = task
            .description
            .as_ref()
            .map(|d| escape_csv(d))
            .unwrap_or_default();
        let created = task.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let completed = task
            .completed_at
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();

        writeln!(
            writer,
            "{id},{title},{status},{priority},{due_date},{tags},{project_id},{description},{created},{completed}"
        )?;
    }

    Ok(())
}

/// Escape a string for CSV (wrap in quotes if needed, escape internal quotes)
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Exports tasks to ICS (iCalendar) format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if writing fails.
pub fn export_to_ics<W: Write>(tasks: &[Task], writer: &mut W) -> std::io::Result<()> {
    // Write calendar header
    writeln!(writer, "BEGIN:VCALENDAR")?;
    writeln!(writer, "VERSION:2.0")?;
    writeln!(writer, "PRODID:-//TaskFlow//TaskFlow TUI//EN")?;
    writeln!(writer, "CALSCALE:GREGORIAN")?;
    writeln!(writer, "METHOD:PUBLISH")?;

    for task in tasks {
        // Export as VTODO (task) component
        writeln!(writer, "BEGIN:VTODO")?;

        // UID (unique identifier)
        let uid = task.id.0;
        writeln!(writer, "UID:{uid}")?;

        // DTSTAMP (timestamp)
        let dtstamp = task.created_at.format("%Y%m%dT%H%M%SZ");
        writeln!(writer, "DTSTAMP:{dtstamp}")?;

        // CREATED
        writeln!(writer, "CREATED:{dtstamp}")?;

        // LAST-MODIFIED
        let last_modified = task.updated_at.format("%Y%m%dT%H%M%SZ");
        writeln!(writer, "LAST-MODIFIED:{last_modified}")?;

        // SUMMARY (title)
        writeln!(writer, "SUMMARY:{}", escape_ics(&task.title))?;

        // DESCRIPTION
        if let Some(ref desc) = task.description {
            writeln!(writer, "DESCRIPTION:{}", escape_ics(desc))?;
        }

        // DUE date
        if let Some(due) = task.due_date {
            writeln!(writer, "DUE;VALUE=DATE:{}", due.format("%Y%m%d"))?;
        }

        // STATUS
        let ics_status = match task.status {
            TaskStatus::Todo => "NEEDS-ACTION",
            TaskStatus::InProgress => "IN-PROCESS",
            TaskStatus::Blocked => "NEEDS-ACTION",
            TaskStatus::Done => "COMPLETED",
            TaskStatus::Cancelled => "CANCELLED",
        };
        writeln!(writer, "STATUS:{ics_status}")?;

        // PRIORITY (1-9 in ICS, 1 is highest)
        let ics_priority = match task.priority {
            Priority::Urgent => 1,
            Priority::High => 3,
            Priority::Medium => 5,
            Priority::Low => 7,
            Priority::None => 9,
        };
        writeln!(writer, "PRIORITY:{ics_priority}")?;

        // COMPLETED timestamp
        if let Some(completed) = task.completed_at {
            writeln!(writer, "COMPLETED:{}", completed.format("%Y%m%dT%H%M%SZ"))?;
        }

        // PERCENT-COMPLETE
        let percent = match task.status {
            TaskStatus::Todo => 0,
            TaskStatus::InProgress => 50,
            TaskStatus::Blocked => 25,
            TaskStatus::Done => 100,
            TaskStatus::Cancelled => 100,
        };
        writeln!(writer, "PERCENT-COMPLETE:{percent}")?;

        // CATEGORIES (tags)
        if !task.tags.is_empty() {
            writeln!(writer, "CATEGORIES:{}", task.tags.join(","))?;
        }

        writeln!(writer, "END:VTODO")?;
    }

    writeln!(writer, "END:VCALENDAR")?;
    Ok(())
}

/// Escape special characters for ICS format
fn escape_ics(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

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
        .map(|(i, id)| (id.clone(), format!("T{i}")))
        .collect();

    writeln!(writer, "    %% Nodes")?;
    for task in tasks.values() {
        let short_id = id_map.get(&task.id).unwrap();
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

/// Exports tasks to a string in the specified format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if formatting fails.
pub fn export_to_string(tasks: &[Task], format: ExportFormat) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    match format {
        ExportFormat::Csv => export_to_csv(tasks, &mut buffer)?,
        ExportFormat::Ics => export_to_ics(tasks, &mut buffer)?,
        ExportFormat::Dot | ExportFormat::Mermaid => {
            // These formats need the full task map for chain/dependency lookups
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Use export_chains_to_string for DOT/Mermaid formats",
            ));
        }
    }
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Exports task chains to a string in DOT or Mermaid format.
///
/// # Errors
///
/// Returns an [`io::Error`](std::io::Error) if formatting fails.
pub fn export_chains_to_string(
    tasks: &HashMap<TaskId, Task>,
    format: ExportFormat,
) -> std::io::Result<String> {
    let mut buffer = Vec::new();
    match format {
        ExportFormat::Dot => export_to_dot(tasks, &mut buffer)?,
        ExportFormat::Mermaid => export_to_mermaid(tasks, &mut buffer)?,
        ExportFormat::Csv | ExportFormat::Ics => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Use export_to_string for CSV/ICS formats",
            ));
        }
    }
    String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};

    fn create_test_task(title: &str) -> Task {
        Task::new(title)
    }

    #[test]
    fn test_export_format_parse() {
        assert_eq!(ExportFormat::parse("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::parse("CSV"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::parse("ics"), Some(ExportFormat::Ics));
        assert_eq!(ExportFormat::parse("ical"), Some(ExportFormat::Ics));
        assert_eq!(ExportFormat::parse("unknown"), None);
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Csv.file_extension(), "csv");
        assert_eq!(ExportFormat::Ics.file_extension(), "ics");
    }

    #[test]
    fn test_escape_csv_simple() {
        assert_eq!(escape_csv("hello"), "hello");
    }

    #[test]
    fn test_escape_csv_with_comma() {
        assert_eq!(escape_csv("hello, world"), "\"hello, world\"");
    }

    #[test]
    fn test_escape_csv_with_quotes() {
        assert_eq!(escape_csv("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_escape_csv_with_newline() {
        assert_eq!(escape_csv("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_escape_ics() {
        assert_eq!(escape_ics("hello"), "hello");
        assert_eq!(escape_ics("a;b"), "a\\;b");
        assert_eq!(escape_ics("a,b"), "a\\,b");
        assert_eq!(escape_ics("a\nb"), "a\\nb");
    }

    #[test]
    fn test_export_csv_basic() {
        let tasks = vec![create_test_task("Test Task")];
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

        assert!(result.starts_with("ID,Title,Status,Priority"));
        assert!(result.contains("Test Task"));
        assert!(result.contains("todo"));
        assert!(result.contains("none"));
    }

    #[test]
    fn test_export_csv_with_due_date() {
        let mut task = create_test_task("Task with date");
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

        assert!(result.contains("2025-06-15"));
    }

    #[test]
    fn test_export_csv_with_tags() {
        let mut task = create_test_task("Tagged task");
        task.tags = vec!["rust".to_string(), "tui".to_string()];

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Csv).unwrap();

        assert!(result.contains("rust;tui"));
    }

    #[test]
    fn test_export_ics_basic() {
        let tasks = vec![create_test_task("ICS Test")];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.starts_with("BEGIN:VCALENDAR"));
        assert!(result.contains("VERSION:2.0"));
        assert!(result.contains("BEGIN:VTODO"));
        assert!(result.contains("SUMMARY:ICS Test"));
        assert!(result.contains("STATUS:NEEDS-ACTION"));
        assert!(result.contains("END:VTODO"));
        assert!(result.ends_with("END:VCALENDAR\n"));
    }

    #[test]
    fn test_export_ics_with_priority() {
        let task = create_test_task("Urgent task").with_priority(Priority::Urgent);
        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("PRIORITY:1"));
    }

    #[test]
    fn test_export_ics_completed() {
        let mut task = create_test_task("Completed task");
        task.status = TaskStatus::Done;
        task.completed_at = Some(Utc::now());

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("STATUS:COMPLETED"));
        assert!(result.contains("PERCENT-COMPLETE:100"));
        assert!(result.contains("COMPLETED:"));
    }

    #[test]
    fn test_export_ics_with_due_date() {
        let mut task = create_test_task("Due task");
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 25).unwrap());

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("DUE;VALUE=DATE:20251225"));
    }

    #[test]
    fn test_export_ics_with_tags() {
        let mut task = create_test_task("Tagged task");
        task.tags = vec!["work".to_string(), "urgent".to_string()];

        let tasks = vec![task];
        let result = export_to_string(&tasks, ExportFormat::Ics).unwrap();

        assert!(result.contains("CATEGORIES:work,urgent"));
    }

    #[test]
    fn test_export_empty_tasks() {
        let tasks: Vec<Task> = vec![];

        let csv_result = export_to_string(&tasks, ExportFormat::Csv).unwrap();
        assert!(csv_result.starts_with("ID,Title,Status")); // Header only

        let ics_result = export_to_string(&tasks, ExportFormat::Ics).unwrap();
        assert!(ics_result.contains("BEGIN:VCALENDAR"));
        assert!(ics_result.contains("END:VCALENDAR"));
        assert!(!ics_result.contains("BEGIN:VTODO")); // No tasks
    }

    #[test]
    fn test_export_format_parse_dot() {
        assert_eq!(ExportFormat::parse("dot"), Some(ExportFormat::Dot));
        assert_eq!(ExportFormat::parse("graphviz"), Some(ExportFormat::Dot));
    }

    #[test]
    fn test_export_format_parse_mermaid() {
        assert_eq!(ExportFormat::parse("mermaid"), Some(ExportFormat::Mermaid));
        assert_eq!(ExportFormat::parse("md"), Some(ExportFormat::Mermaid));
    }

    #[test]
    fn test_escape_dot() {
        assert_eq!(escape_dot("hello"), "hello");
        assert_eq!(escape_dot("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape_dot("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_escape_mermaid() {
        assert_eq!(escape_mermaid("hello"), "hello");
        assert_eq!(escape_mermaid("say \"hi\""), "say 'hi'");
        assert_eq!(escape_mermaid("array[0]"), "array(0)");
    }

    #[test]
    fn test_export_dot_basic() {
        let mut tasks = HashMap::new();
        let task = create_test_task("Test Task");
        tasks.insert(task.id.clone(), task);

        let result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();

        assert!(result.starts_with("digraph TaskChains {"));
        assert!(result.contains("rankdir=LR"));
        assert!(result.contains("Test Task"));
        assert!(result.ends_with("}\n"));
    }

    #[test]
    fn test_export_dot_with_chain() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("First Task");
        let task2 = create_test_task("Second Task");
        let task1_id = task1.id.clone();

        // Create chain: task1 -> task2
        let mut task1_modified = task1.clone();
        task1_modified.next_task_id = Some(task2.id.clone());

        tasks.insert(task1_id.clone(), task1_modified);
        tasks.insert(task2.id.clone(), task2);

        let result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();

        assert!(result.contains("Chain edges"));
        assert!(result.contains("[color=\"blue\" label=\"chain\"]"));
    }

    #[test]
    fn test_export_dot_with_dependency() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("Dependency");
        let task1_id = task1.id.clone();

        let mut task2 = create_test_task("Dependent Task");
        task2.dependencies.push(task1_id.clone());

        tasks.insert(task1_id, task1);
        tasks.insert(task2.id.clone(), task2);

        let result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();

        assert!(result.contains("Dependency edges"));
        assert!(result.contains("[color=\"red\" style=\"dashed\" label=\"blocks\"]"));
    }

    #[test]
    fn test_export_dot_status_colors() {
        let mut tasks = HashMap::new();

        let done_task = create_test_task("Done Task").with_status(TaskStatus::Done);
        tasks.insert(done_task.id.clone(), done_task);

        let result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();

        assert!(result.contains("#90EE90")); // Light green for done
    }

    #[test]
    fn test_export_mermaid_basic() {
        let mut tasks = HashMap::new();
        let task = create_test_task("Test Task");
        tasks.insert(task.id.clone(), task);

        let result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();

        assert!(result.starts_with("```mermaid"));
        assert!(result.contains("flowchart LR"));
        assert!(result.contains("Test Task"));
        assert!(result.ends_with("```\n"));
    }

    #[test]
    fn test_export_mermaid_with_chain() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("First Task");
        let task2 = create_test_task("Second Task");

        let mut task1_modified = task1.clone();
        task1_modified.next_task_id = Some(task2.id.clone());

        tasks.insert(task1.id.clone(), task1_modified);
        tasks.insert(task2.id.clone(), task2);

        let result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();

        assert!(result.contains("Chain edges"));
        assert!(result.contains("-->")); // Chain arrow
    }

    #[test]
    fn test_export_mermaid_with_dependency() {
        let mut tasks = HashMap::new();

        let task1 = create_test_task("Dependency");
        let task1_id = task1.id.clone();

        let mut task2 = create_test_task("Dependent Task");
        task2.dependencies.push(task1_id.clone());

        tasks.insert(task1_id, task1);
        tasks.insert(task2.id.clone(), task2);

        let result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();

        assert!(result.contains("Dependency edges"));
        assert!(result.contains("-.->|blocks|")); // Dependency arrow with label
    }

    #[test]
    fn test_export_mermaid_style_classes() {
        let mut tasks = HashMap::new();
        let done_task = create_test_task("Done Task").with_status(TaskStatus::Done);
        tasks.insert(done_task.id.clone(), done_task);

        let result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();

        assert!(result.contains("classDef done"));
        assert!(result.contains(":::done")); // Node uses the class
    }

    #[test]
    fn test_export_chains_empty() {
        let tasks: HashMap<TaskId, Task> = HashMap::new();

        let dot_result = export_chains_to_string(&tasks, ExportFormat::Dot).unwrap();
        assert!(dot_result.contains("digraph TaskChains"));

        let mermaid_result = export_chains_to_string(&tasks, ExportFormat::Mermaid).unwrap();
        assert!(mermaid_result.contains("flowchart LR"));
    }
}
