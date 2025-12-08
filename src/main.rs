use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::{CommandFactory, Parser, Subcommand, ValueHint};
use clap_complete::{generate, Shell};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use taskflow::app::{
    update, Message, Model, NavigationMessage, PomodoroMessage, RunningState, SystemMessage,
    TaskMessage, TimeMessage, UiMessage,
};
use taskflow::config::{Action, Keybindings, Settings, Theme};
use taskflow::storage::BackendType;
use taskflow::ui::{view, InputMode};

/// `TaskFlow` - A TUI project management application
#[derive(Parser, Debug)]
#[command(name = "taskflow")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to data file or directory
    #[arg(short, long, global = true, value_hint = ValueHint::AnyPath)]
    data: Option<PathBuf>,

    /// Storage backend type
    #[arg(short, long, default_value = "json", global = true, value_enum)]
    backend: BackendType,

    /// Use sample data instead of loading from storage
    #[arg(long, global = true)]
    demo: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate shell completion scripts
    Completion {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Quick add a task from the command line
    #[command(alias = "a")]
    Add {
        /// Task description with optional quick-add syntax
        /// Examples:
        ///   "Buy milk #shopping !high due:tomorrow"
        ///   "Review PR @work #code due:friday"
        #[arg(trailing_var_arg = true, num_args = 1..)]
        task: Vec<String>,
    },
    /// List tasks (without launching TUI)
    #[command(alias = "ls")]
    List {
        /// Filter by view (today, overdue, upcoming, all)
        #[arg(short, long, default_value = "all")]
        view: String,
        /// Show completed tasks
        #[arg(short, long)]
        completed: bool,
        /// Limit number of tasks shown
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
    /// Mark a task as done by searching for it
    #[command(alias = "d")]
    Done {
        /// Search query to find the task (matches title)
        #[arg(trailing_var_arg = true, num_args = 1..)]
        query: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle subcommands
    match &cli.command {
        Some(Commands::Completion { shell }) => {
            print_completions(*shell);
            return Ok(());
        }
        Some(Commands::Add { task }) => {
            return quick_add_task(&cli, task);
        }
        Some(Commands::List {
            view,
            completed,
            limit,
        }) => {
            return list_tasks(&cli, view, *completed, *limit);
        }
        Some(Commands::Done { query }) => {
            return mark_task_done(&cli, query);
        }
        None => {}
    }

    // No subcommand - run the TUI
    run_tui(cli)
}

/// Generate shell completion scripts and print to stdout
fn print_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}

/// Quick add a task from the command line
fn quick_add_task(cli: &Cli, task_words: &[String]) -> anyhow::Result<()> {
    use taskflow::app::quick_add::parse_quick_add;
    use taskflow::domain::Task;

    // Join all words into a single task string
    let task_input = task_words.join(" ");
    if task_input.trim().is_empty() {
        eprintln!("Error: Task description cannot be empty");
        eprintln!("Usage: taskflow add <task description>");
        eprintln!("Example: taskflow add \"Buy milk #shopping !high due:tomorrow\"");
        std::process::exit(1);
    }

    // Load settings and create model with storage
    let settings = Settings::load();
    let backend_type = if cli.backend == BackendType::Json {
        BackendType::parse(&settings.backend).unwrap_or_default()
    } else {
        cli.backend
    };
    let data_path = cli.data.clone().unwrap_or_else(|| settings.get_data_path());

    let mut model = Model::new()
        .with_storage(backend_type, data_path)
        .unwrap_or_else(|_| Model::new());

    // Parse the quick add syntax
    let parsed = parse_quick_add(&task_input);

    // Create the task
    let mut task = Task::new(&parsed.title);

    // Apply parsed metadata
    if let Some(priority) = parsed.priority {
        task = task.with_priority(priority);
    }
    if let Some(due_date) = parsed.due_date {
        task = task.with_due_date(due_date);
    }
    if let Some(sched_date) = parsed.scheduled_date {
        task.scheduled_date = Some(sched_date);
    }
    for tag in &parsed.tags {
        task.tags.push(tag.clone());
    }

    // Find project by name if specified
    if let Some(ref project_name) = parsed.project_name {
        let project_name_lower = project_name.to_lowercase();
        for project in model.projects.values() {
            if project.name.to_lowercase().contains(&project_name_lower) {
                task.project_id = Some(project.id.clone());
                break;
            }
        }
    }

    // Add task and save
    let task_title = task.title.clone();
    let task_id = task.id.clone();
    model.tasks.insert(task_id.clone(), task.clone());

    // Sync to storage
    model.sync_task(&task);
    if let Err(e) = model.save() {
        eprintln!("Warning: Could not save task: {e}");
    }

    // Print confirmation
    println!("✓ Added: {}", task_title);
    if !parsed.tags.is_empty() {
        println!(
            "  Tags: {}",
            parsed
                .tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    if let Some(priority) = parsed.priority {
        println!("  Priority: {:?}", priority);
    }
    if let Some(due) = parsed.due_date {
        println!("  Due: {}", due.format("%Y-%m-%d"));
    }
    if let Some(ref project_name) = parsed.project_name {
        if task.project_id.is_some() {
            println!("  Project: @{project_name}");
        } else {
            println!("  Project: @{project_name} (not found)");
        }
    }

    Ok(())
}

/// List tasks from the command line
fn list_tasks(cli: &Cli, view: &str, show_completed: bool, limit: usize) -> anyhow::Result<()> {
    use chrono::Utc;

    // Load settings and create model with storage
    let settings = Settings::load();
    let backend_type = if cli.backend == BackendType::Json {
        BackendType::parse(&settings.backend).unwrap_or_default()
    } else {
        cli.backend
    };
    let data_path = cli.data.clone().unwrap_or_else(|| settings.get_data_path());

    let model = Model::new()
        .with_storage(backend_type, data_path)
        .unwrap_or_else(|_| Model::new());

    let today = Utc::now().date_naive();

    // Filter tasks based on view
    let mut tasks: Vec<_> = model
        .tasks
        .values()
        .filter(|t| {
            // Filter by completion status
            if !show_completed && t.status.is_complete() {
                return false;
            }

            // Filter by view
            match view.to_lowercase().as_str() {
                "today" => t.due_date.is_some_and(|d| d == today),
                "overdue" => t.due_date.is_some_and(|d| d < today) && !t.status.is_complete(),
                "upcoming" => t
                    .due_date
                    .is_some_and(|d| d > today && d <= today + chrono::Duration::days(7)),
                _ => true, // "all" or any other value
            }
        })
        .collect();

    // Sort by due date, then priority (higher priority first)
    tasks.sort_by(|a, b| {
        match (&a.due_date, &b.due_date) {
            (Some(da), Some(db)) => da.cmp(db),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => {
                // Sort by priority level (urgent > high > medium > low > none)
                let priority_order = |p: &taskflow::domain::Priority| match p {
                    taskflow::domain::Priority::Urgent => 0,
                    taskflow::domain::Priority::High => 1,
                    taskflow::domain::Priority::Medium => 2,
                    taskflow::domain::Priority::Low => 3,
                    taskflow::domain::Priority::None => 4,
                };
                priority_order(&a.priority).cmp(&priority_order(&b.priority))
            }
        }
    });

    // Limit output
    let tasks: Vec<_> = tasks.into_iter().take(limit).collect();

    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    // Print header
    let view_name = match view.to_lowercase().as_str() {
        "today" => "Today's Tasks",
        "overdue" => "Overdue Tasks",
        "upcoming" => "Upcoming Tasks",
        _ => "All Tasks",
    };
    println!("{} ({} shown)", view_name, tasks.len());
    println!("{}", "-".repeat(60));

    // Print tasks
    for task in tasks {
        let status_icon = if task.status.is_complete() {
            "✓"
        } else {
            "○"
        };

        let priority_icon = match task.priority {
            taskflow::domain::Priority::Urgent => "‼️",
            taskflow::domain::Priority::High => "❗",
            taskflow::domain::Priority::Medium => "•",
            taskflow::domain::Priority::Low => "·",
            taskflow::domain::Priority::None => " ",
        };

        let due_str = task
            .due_date
            .map(|d| {
                if d == today {
                    "today".to_string()
                } else if d == today + chrono::Duration::days(1) {
                    "tomorrow".to_string()
                } else if d < today {
                    format!("{} (overdue)", d.format("%m/%d"))
                } else {
                    d.format("%m/%d").to_string()
                }
            })
            .unwrap_or_default();

        let tags_str = if task.tags.is_empty() {
            String::new()
        } else {
            format!(
                " {}",
                task.tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };

        println!(
            "{} {} {}{}{}",
            status_icon,
            priority_icon,
            task.title,
            if due_str.is_empty() {
                String::new()
            } else {
                format!(" [{}]", due_str)
            },
            tags_str
        );
    }

    Ok(())
}

/// Mark a task as done from the command line
fn mark_task_done(cli: &Cli, query_words: &[String]) -> anyhow::Result<()> {
    use taskflow::domain::TaskStatus;

    let query = query_words.join(" ").to_lowercase();
    if query.trim().is_empty() {
        eprintln!("Error: Search query cannot be empty");
        eprintln!("Usage: taskflow done <search query>");
        std::process::exit(1);
    }

    // Load settings and create model with storage
    let settings = Settings::load();
    let backend_type = if cli.backend == BackendType::Json {
        BackendType::parse(&settings.backend).unwrap_or_default()
    } else {
        cli.backend
    };
    let data_path = cli.data.clone().unwrap_or_else(|| settings.get_data_path());

    let mut model = Model::new()
        .with_storage(backend_type, data_path)
        .unwrap_or_else(|_| Model::new());

    // Find matching tasks (case-insensitive title search)
    let matches: Vec<_> = model
        .tasks
        .values()
        .filter(|t| !t.status.is_complete() && t.title.to_lowercase().contains(&query))
        .collect();

    match matches.len() {
        0 => {
            eprintln!("No matching incomplete tasks found for: {}", query);
            std::process::exit(1);
        }
        1 => {
            let task_id = matches[0].id.clone();
            let task_title = matches[0].title.clone();

            // Mark as done
            if let Some(task) = model.tasks.get_mut(&task_id) {
                task.status = TaskStatus::Done;
                task.completed_at = Some(chrono::Utc::now());
                let task_clone = task.clone();
                model.sync_task(&task_clone);
                if let Err(e) = model.save() {
                    eprintln!("Warning: Could not save: {e}");
                }
            }

            println!("✓ Completed: {}", task_title);
        }
        n => {
            println!("Multiple tasks match '{}' ({} found):", query, n);
            for (i, task) in matches.iter().enumerate() {
                let due_str = task
                    .due_date
                    .map(|d| format!(" [{}]", d.format("%m/%d")))
                    .unwrap_or_default();
                println!("  {}. {}{}", i + 1, task.title, due_str);
            }
            eprintln!("\nPlease use a more specific query.");
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Run the TUI application
fn run_tui(cli: Cli) -> anyhow::Result<()> {
    // Load settings from config file
    let settings = Settings::load();

    // CLI args override config file settings
    let backend_type = if cli.backend == BackendType::Json {
        BackendType::parse(&settings.backend).unwrap_or_default()
    } else {
        cli.backend
    };

    // Determine data path (CLI > config > default)
    let data_path = cli.data.unwrap_or_else(|| settings.get_data_path());

    // Create app state
    let mut model = if cli.demo {
        Model::new().with_sample_data()
    } else {
        // Try to load from storage, fall back to sample data on error
        match Model::new().with_storage(backend_type, data_path.clone()) {
            Ok(m) => {
                if m.tasks.is_empty() {
                    // No tasks loaded, use sample data
                    m.with_sample_data()
                } else {
                    m
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not load data from {}: {e}",
                    data_path.display()
                );
                eprintln!("Starting with sample data...");
                Model::new().with_sample_data()
            }
        }
    };

    // Apply settings to model
    model.show_sidebar = settings.show_sidebar;
    model.show_completed = settings.show_completed;
    model.default_priority = settings.default_priority();
    model.refresh_visible_tasks();

    // Check for overdue tasks and show alert at startup
    model.check_overdue_alert();

    // Load keybindings and theme
    let keybindings = Keybindings::load();
    let theme = Theme::load(&settings.theme);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal, &mut model, &keybindings, &settings, &theme);

    // Save before exit if storage is configured
    if model.has_storage() && model.dirty {
        if let Err(e) = model.save() {
            eprintln!("Warning: Could not save data: {e}");
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    model: &mut Model,
    keybindings: &Keybindings,
    settings: &Settings,
    theme: &Theme,
) -> anyhow::Result<()> {
    use std::time::Instant;

    let auto_save_interval = if settings.auto_save_interval > 0 {
        Some(Duration::from_secs(settings.auto_save_interval))
    } else {
        None
    };
    let mut last_save = Instant::now();
    let mut last_pomodoro_tick = Instant::now();

    loop {
        // Draw UI
        terminal.draw(|frame| view(model, frame, theme))?;

        // Check if quitting
        if model.running == RunningState::Quitting {
            break;
        }

        // Auto-save if interval has passed and there are unsaved changes
        if let Some(interval) = auto_save_interval {
            if model.dirty && model.has_storage() && last_save.elapsed() >= interval {
                let _ = model.save();
                last_save = Instant::now();
            }
        }

        // Tick Pomodoro timer every second if active and not paused
        if last_pomodoro_tick.elapsed() >= Duration::from_secs(1) {
            if let Some(ref session) = model.pomodoro_session {
                if !session.paused {
                    update(model, Message::Pomodoro(PomodoroMessage::Tick));
                }
            }
            last_pomodoro_tick = Instant::now();
        }

        // Handle events with timeout for potential async operations
        if event::poll(Duration::from_millis(100))? {
            let message = match event::read()? {
                Event::Key(key) => handle_key_event(key, model, keybindings),
                Event::Resize(width, height) => {
                    Message::System(SystemMessage::Resize { width, height })
                }
                _ => Message::None,
            };

            // Check if this is a PlayMacro message and handle playback
            let playback_messages = if let Message::Ui(UiMessage::PlayMacro(slot)) = &message {
                model.macro_state.get_playback_messages(*slot)
            } else {
                None
            };

            update(model, message);

            // If we got playback messages, replay them
            if let Some(messages) = playback_messages {
                model.macro_state.playing = true;
                for msg in messages {
                    update(model, msg);
                }
                model.macro_state.playing = false;
            }
        }
    }

    Ok(())
}

fn handle_key_event(key: event::KeyEvent, model: &mut Model, keybindings: &Keybindings) -> Message {
    // Handle delete confirmation dialog first
    if model.show_confirm_delete {
        return match key.code {
            KeyCode::Char('y' | 'Y') => Message::Ui(UiMessage::ConfirmDelete),
            KeyCode::Char('n' | 'N') | KeyCode::Esc => Message::Ui(UiMessage::CancelDelete),
            _ => Message::None,
        };
    }

    // Handle import preview dialog
    if model.show_import_preview {
        return match key.code {
            KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
                Message::System(SystemMessage::ConfirmImport)
            }
            KeyCode::Esc | KeyCode::Char('n' | 'N') => Message::System(SystemMessage::CancelImport),
            _ => Message::None,
        };
    }

    // Handle input mode
    if model.input_mode == InputMode::Editing {
        return match key.code {
            KeyCode::Enter => Message::Ui(UiMessage::SubmitInput),
            KeyCode::Esc => Message::Ui(UiMessage::CancelInput),
            KeyCode::Backspace => Message::Ui(UiMessage::InputBackspace),
            KeyCode::Delete => Message::Ui(UiMessage::InputDelete),
            KeyCode::Left => Message::Ui(UiMessage::InputCursorLeft),
            KeyCode::Right => Message::Ui(UiMessage::InputCursorRight),
            KeyCode::Home => Message::Ui(UiMessage::InputCursorStart),
            KeyCode::End => Message::Ui(UiMessage::InputCursorEnd),
            KeyCode::Char(c) => Message::Ui(UiMessage::InputChar(c)),
            _ => Message::None,
        };
    }

    // If overdue alert is showing, any key dismisses it
    if model.show_overdue_alert {
        return Message::Ui(UiMessage::DismissOverdueAlert);
    }

    // If help is showing, any key closes it
    if model.show_help {
        return Message::Ui(UiMessage::HideHelp);
    }

    // If focus mode is active, Esc exits it
    if model.focus_mode && key.code == KeyCode::Esc {
        return Message::Ui(UiMessage::ToggleFocusMode);
    }
    // In focus mode, still allow some keybindings (t, x, f, etc.)
    // Fall through to normal key handling

    // If template picker is showing, handle navigation and selection
    if model.show_templates {
        return match key.code {
            KeyCode::Esc => Message::Ui(UiMessage::HideTemplates),
            KeyCode::Enter => Message::Ui(UiMessage::SelectTemplate(model.template_selected)),
            KeyCode::Up | KeyCode::Char('k') => {
                if model.template_selected > 0 {
                    model.template_selected -= 1;
                }
                Message::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = model.template_manager.len().saturating_sub(1);
                if model.template_selected < max {
                    model.template_selected += 1;
                }
                Message::None
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let index = c.to_digit(10).unwrap() as usize;
                if index < model.template_manager.len() {
                    Message::Ui(UiMessage::SelectTemplate(index))
                } else {
                    Message::None
                }
            }
            _ => Message::None,
        };
    }

    // If keybindings editor is showing, handle navigation and editing
    if model.show_keybindings_editor {
        // If capturing a key, any key except Esc sets the keybinding
        if model.keybinding_capturing {
            return match key.code {
                KeyCode::Esc => Message::Ui(UiMessage::CancelEditKeybinding),
                _ => {
                    let key_str = key_event_to_string(&key);
                    Message::Ui(UiMessage::ApplyKeybinding(key_str))
                }
            };
        }

        // Normal keybindings editor navigation
        return match key.code {
            KeyCode::Esc => Message::Ui(UiMessage::HideKeybindingsEditor),
            KeyCode::Enter => Message::Ui(UiMessage::StartEditKeybinding),
            KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::KeybindingsUp),
            KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::KeybindingsDown),
            KeyCode::Char('r') => Message::Ui(UiMessage::ResetKeybinding),
            KeyCode::Char('R') => Message::Ui(UiMessage::ResetAllKeybindings),
            KeyCode::Char('s') => Message::Ui(UiMessage::SaveKeybindings),
            _ => Message::None,
        };
    }

    // If time log editor is showing, handle navigation and editing
    if model.show_time_log {
        use taskflow::ui::TimeLogMode;

        match model.time_log_mode {
            TimeLogMode::EditStart | TimeLogMode::EditEnd => {
                // Editing time - handle character input
                return match key.code {
                    KeyCode::Esc => Message::Ui(UiMessage::TimeLogCancel),
                    KeyCode::Enter => Message::Ui(UiMessage::TimeLogSubmit),
                    KeyCode::Backspace => Message::Ui(UiMessage::InputBackspace),
                    KeyCode::Char(c) => Message::Ui(UiMessage::InputChar(c)),
                    _ => Message::None,
                };
            }
            TimeLogMode::ConfirmDelete => {
                // Confirm delete mode
                return match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        Message::Ui(UiMessage::TimeLogDelete)
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        Message::Ui(UiMessage::TimeLogCancel)
                    }
                    _ => Message::None,
                };
            }
            TimeLogMode::Browse => {
                // Normal time log navigation
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideTimeLog),
                    KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::TimeLogUp),
                    KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::TimeLogDown),
                    KeyCode::Char('s') => Message::Ui(UiMessage::TimeLogEditStart),
                    KeyCode::Char('e') => Message::Ui(UiMessage::TimeLogEditEnd),
                    KeyCode::Char('d') => Message::Ui(UiMessage::TimeLogConfirmDelete),
                    KeyCode::Char('a') => Message::Ui(UiMessage::TimeLogAddEntry),
                    _ => Message::None,
                };
            }
        }
    }

    // If work log editor is showing, handle navigation and editing
    if model.show_work_log {
        use taskflow::ui::WorkLogMode;

        match model.work_log_mode {
            WorkLogMode::Add | WorkLogMode::Edit => {
                // Multi-line editing mode - handle character input
                // Check for Ctrl+S to save first
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
                    return Message::Ui(UiMessage::WorkLogSubmit);
                }
                return match key.code {
                    KeyCode::Esc => Message::Ui(UiMessage::WorkLogCancel),
                    KeyCode::Enter => Message::Ui(UiMessage::WorkLogNewline),
                    KeyCode::Backspace => Message::Ui(UiMessage::WorkLogInputBackspace),
                    KeyCode::Delete => Message::Ui(UiMessage::WorkLogInputDelete),
                    KeyCode::Left => Message::Ui(UiMessage::WorkLogCursorLeft),
                    KeyCode::Right => Message::Ui(UiMessage::WorkLogCursorRight),
                    KeyCode::Up => Message::Ui(UiMessage::WorkLogCursorUp),
                    KeyCode::Down => Message::Ui(UiMessage::WorkLogCursorDown),
                    KeyCode::Home => Message::Ui(UiMessage::WorkLogCursorHome),
                    KeyCode::End => Message::Ui(UiMessage::WorkLogCursorEnd),
                    KeyCode::Char(c) => Message::Ui(UiMessage::WorkLogInputChar(c)),
                    _ => Message::None,
                };
            }
            WorkLogMode::ConfirmDelete => {
                // Confirm delete mode
                return match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        Message::Ui(UiMessage::WorkLogDelete)
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        Message::Ui(UiMessage::WorkLogCancel)
                    }
                    _ => Message::None,
                };
            }
            WorkLogMode::View => {
                // Viewing a single entry
                return match key.code {
                    KeyCode::Esc | KeyCode::Enter => Message::Ui(UiMessage::WorkLogCancel),
                    KeyCode::Char('e') => Message::Ui(UiMessage::WorkLogEdit),
                    KeyCode::Char('d') => Message::Ui(UiMessage::WorkLogConfirmDelete),
                    _ => Message::None,
                };
            }
            WorkLogMode::Browse => {
                // Normal work log navigation
                return match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => Message::Ui(UiMessage::HideWorkLog),
                    KeyCode::Up | KeyCode::Char('k') => Message::Ui(UiMessage::WorkLogUp),
                    KeyCode::Down | KeyCode::Char('j') => Message::Ui(UiMessage::WorkLogDown),
                    KeyCode::Enter => Message::Ui(UiMessage::WorkLogView),
                    KeyCode::Char('a') => Message::Ui(UiMessage::WorkLogAdd),
                    KeyCode::Char('e') => Message::Ui(UiMessage::WorkLogEdit),
                    KeyCode::Char('d') => Message::Ui(UiMessage::WorkLogConfirmDelete),
                    _ => Message::None,
                };
            }
        }
    }

    // In multi-select mode, Space toggles task selection
    if model.multi_select_mode && key.code == KeyCode::Char(' ') {
        return Message::Ui(UiMessage::ToggleTaskSelection);
    }

    // In calendar view, handle focus switching and navigation
    if model.current_view == taskflow::app::ViewId::Calendar
        && model.focus_pane == taskflow::app::FocusPane::TaskList
    {
        // Tab toggles focus between calendar grid and task list
        if key.code == KeyCode::Tab {
            return if model.calendar_state.focus_task_list {
                Message::Navigation(NavigationMessage::CalendarFocusGrid)
            } else {
                Message::Navigation(NavigationMessage::CalendarFocusTaskList)
            };
        }

        if model.calendar_state.focus_task_list {
            // When focused on task list, h goes back to calendar grid
            match key.code {
                KeyCode::Char('h') | KeyCode::Left => {
                    return Message::Navigation(NavigationMessage::CalendarFocusGrid);
                }
                _ => {}
            }
        } else {
            // When focused on calendar grid, navigate days
            match key.code {
                KeyCode::Left => return Message::Ui(UiMessage::CalendarPrevDay),
                KeyCode::Right => return Message::Ui(UiMessage::CalendarNextDay),
                KeyCode::Char('h') => return Message::Ui(UiMessage::CalendarPrevDay),
                KeyCode::Char('l') => {
                    // l moves to task list if there are tasks, otherwise next day
                    if !model.tasks_for_selected_day().is_empty() {
                        return Message::Navigation(NavigationMessage::CalendarFocusTaskList);
                    }
                    return Message::Ui(UiMessage::CalendarNextDay);
                }
                _ => {}
            }
        }
    }

    // Handle macro slot selection if pending
    if model.pending_macro_slot.is_some() {
        if let KeyCode::Char(c) = key.code {
            if let Some(digit) = c.to_digit(10) {
                let slot = digit as usize;
                model.pending_macro_slot = Some(slot);
                if model.macro_state.is_recording() {
                    // Stop recording and save to this slot
                    return Message::Ui(UiMessage::StopRecordMacro);
                }
                // Start recording to this slot
                return Message::Ui(UiMessage::StartRecordMacro);
            }
        }
        // Escape cancels macro slot selection
        if key.code == KeyCode::Esc {
            model.pending_macro_slot = None;
            model.status_message = Some("Macro cancelled".to_string());
            return Message::None;
        }
    }

    // Convert key event to string for lookup
    let key_str = key_event_to_string(&key);

    // Look up action in keybindings
    if let Some(action) = keybindings.get_action(&key_str) {
        return action_to_message(action);
    }

    Message::None
}

/// Convert a key event to the string format used in keybindings
fn key_event_to_string(key: &event::KeyEvent) -> String {
    let mut parts = Vec::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("ctrl");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("alt");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) && !matches!(key.code, KeyCode::Char(_)) {
        parts.push("shift");
    }

    let key_name = match key.code {
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::F(n) => format!("f{n}"),
        _ => return String::new(),
    };

    if parts.is_empty() {
        key_name
    } else {
        parts.push(&key_name);
        parts.join("+")
    }
}

/// Convert an Action to a Message
const fn action_to_message(action: &Action) -> Message {
    match action {
        Action::MoveUp => Message::Navigation(NavigationMessage::Up),
        Action::MoveDown => Message::Navigation(NavigationMessage::Down),
        Action::MoveFirst => Message::Navigation(NavigationMessage::First),
        Action::MoveLast => Message::Navigation(NavigationMessage::Last),
        Action::PageUp => Message::Navigation(NavigationMessage::PageUp),
        Action::PageDown => Message::Navigation(NavigationMessage::PageDown),
        Action::ToggleComplete => Message::Task(TaskMessage::ToggleComplete),
        Action::CreateTask => Message::Ui(UiMessage::StartCreateTask),
        Action::CreateSubtask => Message::Ui(UiMessage::StartCreateSubtask),
        Action::CreateProject => Message::Ui(UiMessage::StartCreateProject),
        Action::EditProject => Message::Ui(UiMessage::StartEditProject),
        Action::DeleteProject => Message::Ui(UiMessage::DeleteProject),
        Action::EditTask => Message::Ui(UiMessage::StartEditTask),
        Action::EditDueDate => Message::Ui(UiMessage::StartEditDueDate),
        Action::EditScheduledDate => Message::Ui(UiMessage::StartEditScheduledDate),
        Action::EditTags => Message::Ui(UiMessage::StartEditTags),
        Action::EditDescription => Message::Ui(UiMessage::StartEditDescription),
        Action::DeleteTask => Message::Ui(UiMessage::ShowDeleteConfirm),
        Action::CyclePriority => Message::Task(TaskMessage::CyclePriority),
        Action::MoveToProject => Message::Ui(UiMessage::StartMoveToProject),
        Action::ToggleTimeTracking => Message::Time(TimeMessage::ToggleTracking),
        Action::ShowTimeLog => Message::Ui(UiMessage::ShowTimeLog),
        Action::ShowWorkLog => Message::Ui(UiMessage::ShowWorkLog),
        Action::ToggleSidebar => Message::Ui(UiMessage::ToggleSidebar),
        Action::ToggleShowCompleted => Message::Ui(UiMessage::ToggleShowCompleted),
        Action::ShowHelp => Message::Ui(UiMessage::ShowHelp),
        Action::FocusSidebar => Message::Navigation(NavigationMessage::FocusSidebar),
        Action::FocusTaskList => Message::Navigation(NavigationMessage::FocusTaskList),
        Action::Select => Message::Navigation(NavigationMessage::SelectSidebarItem),
        Action::Search => Message::Ui(UiMessage::StartSearch),
        Action::ClearSearch => Message::Ui(UiMessage::ClearSearch),
        Action::FilterByTag => Message::Ui(UiMessage::StartFilterByTag),
        Action::ClearTagFilter => Message::Ui(UiMessage::ClearTagFilter),
        Action::CycleSortField => Message::Ui(UiMessage::CycleSortField),
        Action::ToggleSortOrder => Message::Ui(UiMessage::ToggleSortOrder),
        Action::ToggleMultiSelect => Message::Ui(UiMessage::ToggleMultiSelect),
        Action::ToggleTaskSelection => Message::Ui(UiMessage::ToggleTaskSelection),
        Action::SelectAll => Message::Ui(UiMessage::SelectAll),
        Action::ClearSelection => Message::Ui(UiMessage::ClearSelection),
        Action::BulkDelete => Message::Ui(UiMessage::BulkDelete),
        Action::BulkMoveToProject => Message::Ui(UiMessage::StartBulkMoveToProject),
        Action::BulkSetStatus => Message::Ui(UiMessage::StartBulkSetStatus),
        Action::EditDependencies => Message::Ui(UiMessage::StartEditDependencies),
        Action::EditRecurrence => Message::Ui(UiMessage::StartEditRecurrence),
        Action::MoveTaskUp => Message::Ui(UiMessage::MoveTaskUp),
        Action::MoveTaskDown => Message::Ui(UiMessage::MoveTaskDown),
        Action::LinkTask => Message::Ui(UiMessage::StartLinkTask),
        Action::UnlinkTask => Message::Ui(UiMessage::UnlinkTask),
        Action::CalendarPrevMonth => Message::Navigation(NavigationMessage::CalendarPrevMonth),
        Action::CalendarNextMonth => Message::Navigation(NavigationMessage::CalendarNextMonth),
        Action::CalendarPrevDay => Message::Ui(UiMessage::CalendarPrevDay),
        Action::CalendarNextDay => Message::Ui(UiMessage::CalendarNextDay),
        Action::Save => Message::System(SystemMessage::Save),
        Action::Undo => Message::System(SystemMessage::Undo),
        Action::Redo => Message::System(SystemMessage::Redo),
        Action::Quit => Message::System(SystemMessage::Quit),
        Action::ExportCsv => Message::System(SystemMessage::ExportCsv),
        Action::ExportIcs => Message::System(SystemMessage::ExportIcs),
        Action::ExportChainsDot => Message::System(SystemMessage::ExportChainsDot),
        Action::ExportChainsMermaid => Message::System(SystemMessage::ExportChainsMermaid),
        Action::ExportReportMarkdown => Message::System(SystemMessage::ExportReportMarkdown),
        Action::ExportReportHtml => Message::System(SystemMessage::ExportReportHtml),
        Action::ImportCsv => Message::System(SystemMessage::StartImportCsv),
        Action::ImportIcs => Message::System(SystemMessage::StartImportIcs),
        Action::RecordMacro => Message::Ui(UiMessage::StartRecordMacro),
        Action::StopRecordMacro => Message::Ui(UiMessage::StopRecordMacro),
        Action::PlayMacro0 => Message::Ui(UiMessage::PlayMacro(0)),
        Action::PlayMacro1 => Message::Ui(UiMessage::PlayMacro(1)),
        Action::PlayMacro2 => Message::Ui(UiMessage::PlayMacro(2)),
        Action::PlayMacro3 => Message::Ui(UiMessage::PlayMacro(3)),
        Action::PlayMacro4 => Message::Ui(UiMessage::PlayMacro(4)),
        Action::PlayMacro5 => Message::Ui(UiMessage::PlayMacro(5)),
        Action::PlayMacro6 => Message::Ui(UiMessage::PlayMacro(6)),
        Action::PlayMacro7 => Message::Ui(UiMessage::PlayMacro(7)),
        Action::PlayMacro8 => Message::Ui(UiMessage::PlayMacro(8)),
        Action::PlayMacro9 => Message::Ui(UiMessage::PlayMacro(9)),
        Action::ShowTemplates => Message::Ui(UiMessage::ShowTemplates),
        Action::ToggleFocusMode => Message::Ui(UiMessage::ToggleFocusMode),
        Action::ShowKeybindingsEditor => Message::Ui(UiMessage::ShowKeybindingsEditor),
        Action::SnoozeTask => Message::Ui(UiMessage::StartSnoozeTask),
        Action::ClearSnooze => Message::Ui(UiMessage::ClearSnooze),
        Action::ReportsNextPanel => Message::Navigation(NavigationMessage::ReportsNextPanel),
        Action::ReportsPrevPanel => Message::Navigation(NavigationMessage::ReportsPrevPanel),
        Action::PomodoroStart => Message::Pomodoro(PomodoroMessage::Start { goal_cycles: 4 }),
        Action::PomodoroPause => Message::Pomodoro(PomodoroMessage::Pause),
        Action::PomodoroResume => Message::Pomodoro(PomodoroMessage::Resume),
        Action::PomodoroTogglePause => Message::Pomodoro(PomodoroMessage::TogglePause),
        Action::PomodoroSkip => Message::Pomodoro(PomodoroMessage::Skip),
        Action::PomodoroStop => Message::Pomodoro(PomodoroMessage::Stop),
        Action::RefreshStorage => Message::System(SystemMessage::RefreshStorage),
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        // Validates the CLI configuration is correct
        Cli::command().debug_assert();
    }

    #[test]
    fn test_parse_no_subcommand() {
        let cli = Cli::try_parse_from(["taskflow"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.backend, BackendType::Json);
        assert!(!cli.demo);
    }

    #[test]
    fn test_parse_completion_bash() {
        let cli = Cli::try_parse_from(["taskflow", "completion", "bash"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completion { shell: Shell::Bash })
        ));
    }

    #[test]
    fn test_parse_completion_zsh() {
        let cli = Cli::try_parse_from(["taskflow", "completion", "zsh"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completion { shell: Shell::Zsh })
        ));
    }

    #[test]
    fn test_parse_completion_fish() {
        let cli = Cli::try_parse_from(["taskflow", "completion", "fish"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completion { shell: Shell::Fish })
        ));
    }

    #[test]
    fn test_parse_with_backend() {
        let cli = Cli::try_parse_from(["taskflow", "--backend", "yaml"]).unwrap();
        assert_eq!(cli.backend, BackendType::Yaml);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_parse_with_data_path() {
        let cli = Cli::try_parse_from(["taskflow", "--data", "/tmp/tasks.json"]).unwrap();
        assert_eq!(cli.data, Some(PathBuf::from("/tmp/tasks.json")));
    }

    #[test]
    fn test_parse_with_demo() {
        let cli = Cli::try_parse_from(["taskflow", "--demo"]).unwrap();
        assert!(cli.demo);
    }

    #[test]
    fn test_parse_all_backends() {
        for backend in ["json", "yaml", "sqlite", "markdown"] {
            let cli = Cli::try_parse_from(["taskflow", "--backend", backend]).unwrap();
            let expected = BackendType::parse(backend).unwrap();
            assert_eq!(cli.backend, expected);
        }
    }

    #[test]
    fn test_completion_output_bash() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Bash, &mut cmd, "taskflow", &mut buf);
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("_taskflow"));
        assert!(output.contains("--backend"));
        assert!(output.contains("--data"));
        assert!(output.contains("completion"));
    }

    #[test]
    fn test_completion_output_zsh() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Zsh, &mut cmd, "taskflow", &mut buf);
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("#compdef taskflow"));
        assert!(output.contains("--backend"));
    }

    #[test]
    fn test_completion_output_fish() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Fish, &mut cmd, "taskflow", &mut buf);
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("complete -c taskflow"));
    }

    #[test]
    fn test_invalid_backend_rejected() {
        let result = Cli::try_parse_from(["taskflow", "--backend", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_shell_rejected() {
        let result = Cli::try_parse_from(["taskflow", "completion", "invalid"]);
        assert!(result.is_err());
    }
}
