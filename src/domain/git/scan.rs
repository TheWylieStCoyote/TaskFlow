//! Git TODO scanning logic.
//!
//! Provides functionality to scan a git repository for TODO/FIXME/HACK comments
//! using `git grep` and return structured results.

use std::path::Path;
use std::process::Command;

/// A TODO comment extracted from a git repository.
#[derive(Debug, Clone)]
pub struct GitTodoItem {
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub title: String,
    pub context: String,
}

/// Default patterns used when scanning without explicit configuration.
pub const DEFAULT_PATTERNS: &[&str] = &["TODO", "FIXME", "HACK"];

/// Scan a git repository for TODO/FIXME/HACK comments using `git grep`.
///
/// Uses the default patterns (`TODO`, `FIXME`, `HACK`). Returns an empty vec
/// if the directory is not a git repository or `git grep` fails.
#[must_use]
pub fn scan_git_todos(repo: &Path) -> Vec<GitTodoItem> {
    let patterns: Vec<String> = DEFAULT_PATTERNS.iter().map(ToString::to_string).collect();
    scan_git_todos_with_patterns(repo, &patterns)
}

/// Scan a git repository for TODO comments with custom patterns.
///
/// Returns an empty vec if the directory is not a git repository or `git grep` fails.
#[must_use]
pub fn scan_git_todos_with_patterns(repo: &Path, patterns: &[String]) -> Vec<GitTodoItem> {
    let pattern = patterns.join("\\|");

    let Ok(output) = Command::new("git")
        .args(["-C", &repo.to_string_lossy(), "grep", "-n", "-I", &pattern])
        .output()
    else {
        return Vec::new();
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut todos = Vec::new();

    for line in stdout.lines() {
        if let Some(todo) = parse_git_grep_line(line, patterns) {
            todos.push(todo);
        }
    }

    todos
}

/// Parse a line from git grep output.
/// Format: `file:line:content`
pub(crate) fn parse_git_grep_line(line: &str, patterns: &[String]) -> Option<GitTodoItem> {
    let mut parts = line.splitn(3, ':');
    let file = parts.next()?.to_string();
    let line_num: usize = parts.next()?.parse().ok()?;
    let content = parts.next()?.to_string();

    if !is_comment_line(&content) {
        return None;
    }

    let comment_body = get_comment_body(&content);

    let body_upper = comment_body.to_uppercase();
    let pattern = patterns
        .iter()
        .find(|p| {
            let p_upper = p.to_uppercase();
            if !body_upper.starts_with(&p_upper) {
                return false;
            }
            let after_pos = p.len();
            if after_pos < comment_body.len() {
                let after_char = comment_body.chars().nth(after_pos).unwrap_or(' ');
                after_char == ':'
                    || after_char == '-'
                    || after_char == '('
                    || after_char == '['
                    || after_char.is_whitespace()
            } else {
                false
            }
        })?
        .clone();

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
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
        || trimmed.starts_with("--")
        || trimmed.starts_with("<!--")
        || trimmed.starts_with("rem ")
        || trimmed.starts_with(';')
        || trimmed.starts_with('%')
}

/// Get the body of a comment (content after the comment marker).
fn get_comment_body(content: &str) -> &str {
    let trimmed = content.trim();
    let markers = [
        "///", "//!", "//", "<!--", "-->", "/*", "*/", "*", "##", "#", "--", "rem ", ";", "%",
    ];
    for marker in markers {
        if let Some(rest) = trimmed.strip_prefix(marker) {
            return rest.trim_start();
        }
    }
    trimmed
}

/// Extract a meaningful title from a TODO comment.
fn extract_todo_title(content: &str, pattern: &str) -> String {
    let content = content.trim();
    let pattern_upper = pattern.to_uppercase();
    let content_upper = content.to_uppercase();

    if let Some(pos) = content_upper.find(&pattern_upper) {
        let mut after_pattern = &content[pos + pattern.len()..];

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

        let title =
            after_pattern.trim_start_matches(|c: char| c == ':' || c == '-' || c.is_whitespace());
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
            assert_eq!(result.title, expected_title, "Failed for: {line}");
        }
    }

    #[test]
    fn test_parse_git_grep_line_rejects_non_todo_comments() {
        let patterns = vec!["TODO".to_string(), "FIXME".to_string()];

        assert!(parse_git_grep_line(
            "file.rs:1:/// Extract TODO/FIXME comments from repo",
            &patterns
        )
        .is_none());

        assert!(
            parse_git_grep_line("file.rs:1:let todo = \"TODO: something\"", &patterns).is_none()
        );

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
    fn test_parse_invalid_lines() {
        let patterns = vec!["TODO".to_string()];

        assert!(parse_git_grep_line("file.rs:content", &patterns).is_none());
        assert!(parse_git_grep_line("file.rs:1:no pattern here", &patterns).is_none());
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
