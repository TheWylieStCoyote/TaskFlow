//! Git TODO extraction command.

use std::collections::HashMap;
use std::path::Path;

use tracing::warn;

use taskflow::app::extract_git_location;
use taskflow::domain::git::scan_git_todos_with_patterns;
use taskflow::domain::{Priority, Task};

use crate::cli::Cli;
use crate::load_model_for_cli;

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
    let todos = scan_git_todos_with_patterns(repo, patterns);

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
                if task.project_id.is_none() {
                    task.project_id = project_id;
                }
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
        warn!(error = %e, "Could not save git TODOs to disk");
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
            warn!(project = %name, "Referenced project not found for git TODOs");
            eprintln!("  Project: @{} (not found)", name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use taskflow::app::{extract_git_location, Model};
    use taskflow::domain::{Priority, Project, Task};

    use super::extract_git_todos;
    use crate::commands_tests::support::{
        add_project, commit_file, data_file, git_available, init_repo, open_model, seed_model,
        temp_dir, yaml_cli,
    };

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

    struct Fixture {
        _dir: TempDir,
        repo: PathBuf,
        data: PathBuf,
    }

    /// A repository with two annotated comments on tracked files, or `None`
    /// when git cannot run on this machine.
    fn fixture() -> Option<Fixture> {
        if !git_available() {
            eprintln!("git binary not available; skipping");
            return None;
        }
        let dir = temp_dir();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        init_repo(&repo);
        commit_file(
            &repo,
            "src/lib.rs",
            "fn a() {}\n// TODO: refactor parser\n// FIXME(wylie): handle errors\nlet todo = 1; // TODO not on a comment line\n",
            "Add annotated source",
        );
        commit_file(&repo, "notes.py", "# HACK: temporary\n", "Add notes");
        let data = data_file(dir.path());
        Some(Fixture {
            _dir: dir,
            repo,
            data,
        })
    }

    fn patterns(items: &[&str]) -> Vec<String> {
        items.iter().map(ToString::to_string).collect()
    }

    fn task_named<'a>(model: &'a Model, title: &str) -> &'a Task {
        model
            .tasks
            .values()
            .find(|t| t.title == title)
            .unwrap_or_else(|| panic!("task {title} exists"))
    }

    fn extract(
        f: &Fixture,
        project: Option<&str>,
        tags: Option<&[String]>,
        priority: &str,
        dry_run: bool,
    ) -> anyhow::Result<()> {
        extract_git_todos(
            &yaml_cli(&f.data),
            &f.repo,
            &patterns(&["TODO", "FIXME"]),
            project,
            tags,
            priority,
            dry_run,
        )
    }

    #[test]
    fn dry_run_touches_no_data() {
        let Some(f) = fixture() else { return };
        extract(&f, None, None, "medium", true).expect("dry run succeeds");
        assert!(!f.data.exists(), "dry run never opens storage");
    }

    #[test]
    fn no_matches_touches_no_data() {
        let Some(f) = fixture() else { return };
        extract_git_todos(
            &yaml_cli(&f.data),
            &f.repo,
            &patterns(&["XYZZY"]),
            None,
            None,
            "medium",
            false,
        )
        .expect("no matches is not an error");
        assert!(!f.data.exists());

        let not_a_repo = f._dir.path().join("plain");
        std::fs::create_dir_all(&not_a_repo).unwrap();
        extract_git_todos(
            &yaml_cli(&f.data),
            &not_a_repo,
            &patterns(&["TODO"]),
            None,
            None,
            "medium",
            false,
        )
        .expect("a non-repository yields no matches");
        assert!(!f.data.exists());
    }

    #[test]
    fn creates_one_task_per_comment_with_tags_priority_and_location() {
        let Some(f) = fixture() else { return };
        let mut backend_id = None;
        seed_model(&f.data, |model| {
            backend_id = Some(add_project(model, Project::new("Backend")));
        });

        extract(&f, Some("back"), Some(&patterns(&["scan"])), "high", false)
            .expect("extraction succeeds");

        let model = open_model(&f.data);
        assert_eq!(
            model.tasks.len(),
            2,
            "the HACK line and code line are ignored"
        );

        let todo = task_named(&model, "refactor parser");
        assert_eq!(
            todo.tags,
            vec![
                "git-todo".to_string(),
                "todo".to_string(),
                "scan".to_string()
            ]
        );
        assert_eq!(todo.priority, Priority::High);
        assert_eq!(todo.project_id, backend_id);
        let desc = todo.description.as_deref().unwrap();
        assert!(desc.starts_with("git:src/lib.rs:2\n"), "{desc}");
        assert!(desc.contains("Pattern: TODO"));
        assert_eq!(
            extract_git_location(desc),
            Some(("src/lib.rs".to_string(), 2))
        );

        let fixme = task_named(&model, "handle errors");
        assert_eq!(
            fixme.tags[..2],
            ["git-todo".to_string(), "fixme".to_string()]
        );
        assert_eq!(
            extract_git_location(fixme.description.as_deref().unwrap()),
            Some(("src/lib.rs".to_string(), 3))
        );
    }

    #[test]
    fn rerun_updates_existing_tasks_instead_of_duplicating() {
        let Some(f) = fixture() else { return };
        seed_model(&f.data, |_| {});
        extract(&f, None, None, "low", false).expect("first run");

        let mut backend_id = None;
        seed_model(&f.data, |model| {
            backend_id = Some(add_project(model, Project::new("Backend")));
        });
        extract(
            &f,
            Some("backend"),
            Some(&patterns(&["second"])),
            "urgent",
            false,
        )
        .expect("second run");

        let model = open_model(&f.data);
        assert_eq!(
            model.tasks.len(),
            2,
            "same locations are updated, not recreated"
        );
        for task in model.tasks.values() {
            assert!(task.tags.contains(&"second".to_string()), "{:?}", task.tags);
            assert_eq!(task.tags.iter().filter(|t| *t == "git-todo").count(), 1);
            assert_eq!(
                task.priority,
                Priority::Low,
                "priority of an existing task is left alone"
            );
            assert_eq!(
                task.project_id, backend_id,
                "a project is assigned when the task had none"
            );
        }
    }

    #[test]
    fn unknown_project_leaves_tasks_unassigned() {
        let Some(f) = fixture() else { return };
        seed_model(&f.data, |model| {
            add_project(model, Project::new("Backend"));
        });

        extract(&f, Some("nowhere"), None, "medium", false).expect("extraction succeeds");

        let model = open_model(&f.data);
        assert_eq!(model.tasks.len(), 2);
        assert!(model.tasks.values().all(|t| t.project_id.is_none()));
    }

    #[test]
    fn priority_strings_are_parsed_case_insensitively() {
        let Some(f) = fixture() else { return };
        let cases = [
            ("URGENT", Priority::Urgent),
            ("Medium", Priority::Medium),
            ("med", Priority::Medium),
            ("low", Priority::Low),
            ("bogus", Priority::None),
        ];
        for (input, expected) in cases {
            let data = f._dir.path().join(format!("{input}.yaml"));
            let cli = yaml_cli(&data);
            extract_git_todos(
                &cli,
                &f.repo,
                &patterns(&["TODO"]),
                None,
                None,
                input,
                false,
            )
            .expect("extraction succeeds");
            let model = open_model(&data);
            assert_eq!(model.tasks.len(), 1);
            assert_eq!(
                model.tasks.values().next().unwrap().priority,
                expected,
                "priority string {input}"
            );
        }
    }

    #[test]
    fn single_pattern_extracts_only_that_pattern() {
        let Some(f) = fixture() else { return };
        extract_git_todos(
            &yaml_cli(&f.data),
            &f.repo,
            &patterns(&["FIXME"]),
            None,
            None,
            "none",
            false,
        )
        .expect("extraction succeeds");
        let model = open_model(&f.data);
        assert_eq!(model.tasks.len(), 1);
        assert_eq!(model.tasks.values().next().unwrap().title, "handle errors");
    }
}
