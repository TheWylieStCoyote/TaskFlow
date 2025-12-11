//! Git TODO extraction command.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use taskflow::app::extract_git_location;
use taskflow::domain::{Priority, Task};

use crate::cli::Cli;
use crate::load_model_for_cli;

/// A TODO comment extracted from git
#[derive(Debug, Clone)]
pub struct GitTodoItem {
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub title: String,
    pub context: String,
}

/// Extract TODO/FIXME comments from a git repository and create tasks.
pub fn extract_git_todos(
    cli: &Cli,
    repo: &Path,
    patterns: &[String],
    project_name: Option<&str>,
    extra_tags: Option<&[String]>,
    priority_str: &str,
    dry_run: bool,
) -> anyhow::Result<()> {
    // Parse priority
    let priority = match priority_str.to_lowercase().as_str() {
        "urgent" => Priority::Urgent,
        "high" => Priority::High,
        "medium" | "med" => Priority::Medium,
        "low" => Priority::Low,
        _ => Priority::None,
    };

    // Scan git repository for TODOs
    let todos = scan_git_todos(repo, patterns)?;

    if todos.is_empty() {
        println!("No TODO/FIXME comments found in {}", repo.display());
        return Ok(());
    }

    if dry_run {
        println!("Found {} TODO/FIXME comments (dry run):\n", todos.len());
        for todo in &todos {
            println!("  {} [{}:{}]", todo.title, todo.file, todo.line);
        }
        return Ok(());
    }

    // Load model with storage
    let mut model = load_model_for_cli(cli)?;

    // Find project ID if specified
    let project_id = project_name.and_then(|name| {
        let name_lower = name.to_lowercase();
        model
            .projects
            .values()
            .find(|p| p.name.to_lowercase().contains(&name_lower))
            .map(|p| p.id)
    });

    // Build lookup of existing git-todo tasks by their source location
    let mut existing_by_location: HashMap<String, taskflow::domain::TaskId> = HashMap::new();
    for task in model.tasks.values() {
        if let Some(ref desc) = task.description {
            // Look for "git:<file>:<line>" marker in description
            if let Some((file, line)) = extract_git_location(desc) {
                existing_by_location.insert(format!("{file}:{line}"), task.id);
            }
        }
    }

    let mut created = 0;
    let mut updated = 0;

    for todo in &todos {
        let location_key = format!("{}:{}", todo.file, todo.line);
        let description = format!(
            "git:{}:{}\n\nFile: {}\nLine: {}\nPattern: {}\n\n{}",
            todo.file, todo.line, todo.file, todo.line, todo.pattern, todo.context
        );

        // Build tags
        let mut tags = vec!["git-todo".to_string(), todo.pattern.to_lowercase()];
        if let Some(extra) = extra_tags {
            tags.extend(extra.iter().cloned());
        }

        if let Some(&existing_id) = existing_by_location.get(&location_key) {
            // Update existing task
            if let Some(task) = model.tasks.get_mut(&existing_id) {
                task.title = todo.title.clone();
                task.description = Some(description);
                // Preserve existing priority, status, project unless not set
                if task.project_id.is_none() {
                    task.project_id = project_id;
                }
                // Add new tags if not present
                for tag in &tags {
                    if !task.tags.contains(tag) {
                        task.tags.push(tag.clone());
                    }
                }
                let task_clone = task.clone();
                model.sync_task(&task_clone);
                updated += 1;
            }
        } else {
            // Create new task
            let mut task = Task::new(&todo.title)
                .with_priority(priority)
                .with_description(description);

            if let Some(pid) = project_id {
                task = task.with_project(pid);
            }

            task.tags = tags;

            let task_id = task.id;
            model.tasks.insert(task_id, task.clone());
            model.sync_task(&task);
            created += 1;
        }
    }

    // Save to disk
    if let Err(e) = model.save() {
        eprintln!("Warning: Could not save to disk: {e}");
    }

    // Print summary
    println!("✓ Extracted TODOs from {}", repo.display());
    println!("  Patterns: {}", patterns.join(", "));
    println!("  Created: {}", created);
    println!("  Updated: {}", updated);
    if let Some(name) = project_name {
        if project_id.is_some() {
            println!("  Project: @{}", name);
        } else {
            eprintln!("  Project: @{} (not found)", name);
        }
    }

    Ok(())
}

/// Scan a git repository for TODO/FIXME comments using `git grep`.
fn scan_git_todos(repo: &Path, patterns: &[String]) -> anyhow::Result<Vec<GitTodoItem>> {
    // Build regex pattern for git grep
    let pattern = patterns.join("\\|");

    // Run git grep
    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "grep", "-n", "-I", &pattern])
        .output()?;

    if !output.status.success() && !output.stdout.is_empty() {
        // git grep returns exit code 1 when no matches found
        // but we may have partial output
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut todos = Vec::new();

    for line in stdout.lines() {
        if let Some(todo) = parse_git_grep_line(line, patterns) {
            todos.push(todo);
        }
    }

    Ok(todos)
}

/// Parse a line from git grep output.
/// Format: `file:line:content`
fn parse_git_grep_line(line: &str, patterns: &[String]) -> Option<GitTodoItem> {
    // Split on first two colons: file:line:content
    let mut parts = line.splitn(3, ':');
    let file = parts.next()?.to_string();
    let line_num: usize = parts.next()?.parse().ok()?;
    let content = parts.next()?.to_string();

    // Only match actual code comments, not arbitrary text
    // Look for comment markers before the pattern
    if !is_comment_line(&content) {
        return None;
    }

    // Get the comment content (after the comment marker)
    let comment_body = get_comment_body(&content);

    // Find which pattern matched - must be at the START of the comment body
    // This filters out doc comments that just mention TODO/FIXME in text
    let body_upper = comment_body.to_uppercase();
    let pattern = patterns
        .iter()
        .find(|p| {
            let p_upper = p.to_uppercase();
            // Pattern must be at the start of the comment body
            if !body_upper.starts_with(&p_upper) {
                return false;
            }

            let after_pos = p.len();
            // Require a separator after the pattern (: - ( [ or whitespace)
            if after_pos < comment_body.len() {
                let after_char = comment_body.chars().nth(after_pos).unwrap_or(' ');
                after_char == ':'
                    || after_char == '-'
                    || after_char == '('
                    || after_char == '['
                    || after_char.is_whitespace()
            } else {
                false // Pattern at end of line with no message isn't useful
            }
        })?
        .clone();

    // Extract title from the comment
    let title = extract_todo_title(&content, &pattern);

    Some(GitTodoItem {
        file,
        line: line_num,
        pattern,
        title,
        context: content.trim().to_string(),
    })
}

/// Check if a line appears to be a code comment.
fn is_comment_line(content: &str) -> bool {
    let trimmed = content.trim();

    // Common comment prefixes
    trimmed.starts_with("//")       // C, C++, Rust, Java, JS, Go
        || trimmed.starts_with('#')  // Python, Ruby, Shell, YAML
        || trimmed.starts_with("/*") // C-style block comment start
        || trimmed.starts_with('*')  // C-style block comment continuation
        || trimmed.starts_with("--") // SQL, Haskell, Lua
        || trimmed.starts_with("<!--") // HTML/XML
        || trimmed.starts_with("rem ") // Batch
        || trimmed.starts_with(';')  // Lisp, Assembly, INI
        || trimmed.starts_with('%') // LaTeX, Erlang
}

/// Get the body of a comment (content after the comment marker).
fn get_comment_body(content: &str) -> &str {
    let trimmed = content.trim();

    // Strip comment markers in order of length (longest first)
    let markers = [
        "///", "//!", "//", // Rust doc comments and regular
        "<!--", "-->", // HTML
        "/*", "*/", "*", // C-style
        "##", "#",    // Python/Shell
        "--",   // SQL/Haskell
        "rem ", // Batch
        ";",    // Lisp/ASM
        "%",    // LaTeX
    ];

    for marker in markers {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return rest.trim_start();
        }
    }

    trimmed
}

/// Extract a meaningful title from a TODO comment.
/// Input: `    // TODO: Fix authentication bug`
/// Output: `Fix authentication bug`
///
/// Also handles annotations like `TODO(user): message` → `message`
fn extract_todo_title(content: &str, pattern: &str) -> String {
    let content = content.trim();

    // Find the pattern (case-insensitive)
    let pattern_upper = pattern.to_uppercase();
    let content_upper = content.to_uppercase();

    if let Some(pos) = content_upper.find(&pattern_upper) {
        let mut after_pattern = &content[pos + pattern.len()..];

        // Skip parenthesized annotations like (user) or [issue-123]
        after_pattern = after_pattern.trim_start();
        if after_pattern.starts_with('(') {
            if let Some(close) = after_pattern.find(')') {
                after_pattern = &after_pattern[close + 1..];
            }
        } else if after_pattern.starts_with('[') {
            if let Some(close) = after_pattern.find(']') {
                after_pattern = &after_pattern[close + 1..];
            }
        }

        // Skip common separators: :, -, whitespace
        let title =
            after_pattern.trim_start_matches(|c: char| c == ':' || c == '-' || c.is_whitespace());

        // Clean up trailing comment markers
        let title = title.trim_end_matches("*/").trim_end_matches("-->").trim();

        if title.is_empty() {
            format!(
                "{} in {}",
                pattern,
                content.chars().take(50).collect::<String>()
            )
        } else {
            title.to_string()
        }
    } else {
        content.chars().take(80).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_grep_line() {
        let patterns = vec!["TODO".to_string(), "FIXME".to_string()];

        let line = "src/auth.rs:42:    // TODO: Fix authentication bug";
        let result = parse_git_grep_line(line, &patterns).unwrap();
        assert_eq!(result.file, "src/auth.rs");
        assert_eq!(result.line, 42);
        assert_eq!(result.pattern, "TODO");
        assert_eq!(result.title, "Fix authentication bug");

        let line = "src/api.rs:108:    // FIXME: Handle edge case";
        let result = parse_git_grep_line(line, &patterns).unwrap();
        assert_eq!(result.file, "src/api.rs");
        assert_eq!(result.line, 108);
        assert_eq!(result.pattern, "FIXME");
        assert_eq!(result.title, "Handle edge case");
    }

    #[test]
    fn test_parse_git_grep_line_various_formats() {
        let patterns = vec!["TODO".to_string()];

        // Different comment styles - TODO must be at start of comment body
        let cases = vec![
            ("file.rs:1:// TODO: message", "message"),
            ("file.rs:1:/* TODO: message */", "message"),
            ("file.rs:1:# TODO: message", "message"),
            ("file.rs:1:// TODO - message", "message"),
            ("file.rs:1:// TODO(user): message", "message"),
            ("file.rs:1:// todo: lowercase", "lowercase"),
        ];

        for (line, expected_title) in cases {
            let result = parse_git_grep_line(line, &patterns).unwrap();
            assert_eq!(result.title, expected_title, "Failed for: {}", line);
        }
    }

    #[test]
    fn test_parse_git_grep_line_rejects_non_todo_comments() {
        let patterns = vec!["TODO".to_string(), "FIXME".to_string()];

        // Doc comments that mention TODO/FIXME in text should be rejected
        assert!(parse_git_grep_line(
            "file.rs:1:/// Extract TODO/FIXME comments from repo",
            &patterns
        )
        .is_none());

        // Non-comment lines should be rejected
        assert!(
            parse_git_grep_line("file.rs:1:let todo = \"TODO: something\"", &patterns).is_none()
        );

        // TODO in middle of comment should be rejected
        assert!(parse_git_grep_line("file.rs:1:// This is about TODO items", &patterns).is_none());
    }

    #[test]
    fn test_extract_todo_title() {
        assert_eq!(
            extract_todo_title("// TODO: Fix this bug", "TODO"),
            "Fix this bug"
        );
        assert_eq!(
            extract_todo_title("/* FIXME - broken */", "FIXME"),
            "broken"
        );
        assert_eq!(
            extract_todo_title("# TODO: implement feature", "TODO"),
            "implement feature"
        );
        // Parenthesized annotations should be skipped
        assert_eq!(
            extract_todo_title("// TODO(user): message here", "TODO"),
            "message here"
        );
        assert_eq!(
            extract_todo_title("// TODO[issue-123]: fix the bug", "TODO"),
            "fix the bug"
        );
        assert_eq!(
            extract_todo_title("// FIXME(team) - urgent fix needed", "FIXME"),
            "urgent fix needed"
        );
    }

    #[test]
    fn test_extract_git_location() {
        let desc = "git:src/auth.rs:42\n\nFile: src/auth.rs\nLine: 42";
        assert_eq!(
            extract_git_location(desc),
            Some(("src/auth.rs".to_string(), 42))
        );

        let desc = "No git marker here";
        assert_eq!(extract_git_location(desc), None);
    }

    #[test]
    fn test_parse_invalid_lines() {
        let patterns = vec!["TODO".to_string()];

        // Invalid format - no line number
        assert!(parse_git_grep_line("file.rs:content", &patterns).is_none());

        // No matching pattern
        assert!(parse_git_grep_line("file.rs:1:no pattern here", &patterns).is_none());

        // Invalid line number
        assert!(parse_git_grep_line("file.rs:abc:// TODO: test", &patterns).is_none());
    }

    #[test]
    fn test_is_comment_line() {
        assert!(is_comment_line("// comment"));
        assert!(is_comment_line("   // indented"));
        assert!(is_comment_line("# python comment"));
        assert!(is_comment_line("/* block comment */"));
        assert!(is_comment_line("* continuation"));
        assert!(is_comment_line("-- sql comment"));
        assert!(!is_comment_line("let x = 1;"));
        assert!(!is_comment_line("fn main() {}"));
    }

    #[test]
    fn test_get_comment_body() {
        assert_eq!(get_comment_body("// TODO: test"), "TODO: test");
        assert_eq!(get_comment_body("/// Doc comment"), "Doc comment");
        assert_eq!(get_comment_body("# Python TODO: fix"), "Python TODO: fix");
        assert_eq!(get_comment_body("/* Block */"), "Block */");
        assert_eq!(get_comment_body("   // Indented"), "Indented");
    }
}
