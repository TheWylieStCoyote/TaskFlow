use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use taskflow::app::{
    update, Message, Model, NavigationMessage, RunningState, SystemMessage, TaskMessage,
    TimeMessage, UiMessage,
};
use taskflow::config::{Action, Keybindings, Settings, Theme};
use taskflow::storage::BackendType;
use taskflow::ui::{view, InputMode};

/// TaskFlow - A TUI project management application
#[derive(Parser, Debug)]
#[command(name = "taskflow")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to data file or directory
    #[arg(short, long)]
    data: Option<PathBuf>,

    /// Storage backend type (json, yaml, sqlite, markdown)
    #[arg(short, long, default_value = "json")]
    backend: String,

    /// Use sample data instead of loading from storage
    #[arg(long)]
    demo: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Load settings from config file
    let settings = Settings::load();

    // CLI args override config file settings
    let backend_str = if args.backend != "json" {
        &args.backend
    } else {
        &settings.backend
    };

    // Determine data path (CLI > config > default)
    let data_path = args
        .data
        .clone()
        .unwrap_or_else(|| settings.get_data_path());

    // Parse backend type
    let backend_type = BackendType::parse(backend_str).unwrap_or_default();

    // Create app state
    let mut model = if args.demo {
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
                eprintln!("Warning: Could not load data from {:?}: {}", data_path, e);
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
            eprintln!("Warning: Could not save data: {}", e);
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

        // Handle events with timeout for potential async operations
        if event::poll(Duration::from_millis(100))? {
            let message = match event::read()? {
                Event::Key(key) => handle_key_event(key, model, keybindings),
                Event::Resize(width, height) => {
                    Message::System(SystemMessage::Resize { width, height })
                }
                _ => Message::None,
            };

            update(model, message);
        }
    }

    Ok(())
}

fn handle_key_event(key: event::KeyEvent, model: &Model, keybindings: &Keybindings) -> Message {
    // Handle delete confirmation dialog first
    if model.show_confirm_delete {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Message::Ui(UiMessage::ConfirmDelete),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                Message::Ui(UiMessage::CancelDelete)
            }
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

    // If help is showing, any key closes it
    if model.show_help {
        return Message::Ui(UiMessage::HideHelp);
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
        KeyCode::F(n) => format!("f{}", n),
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
fn action_to_message(action: &Action) -> Message {
    match action {
        Action::MoveUp => Message::Navigation(NavigationMessage::Up),
        Action::MoveDown => Message::Navigation(NavigationMessage::Down),
        Action::MoveFirst => Message::Navigation(NavigationMessage::First),
        Action::MoveLast => Message::Navigation(NavigationMessage::Last),
        Action::PageUp => Message::Navigation(NavigationMessage::PageUp),
        Action::PageDown => Message::Navigation(NavigationMessage::PageDown),
        Action::ToggleComplete => Message::Task(TaskMessage::ToggleComplete),
        Action::CreateTask => Message::Ui(UiMessage::StartCreateTask),
        Action::CreateProject => Message::Ui(UiMessage::StartCreateProject),
        Action::EditTask => Message::Ui(UiMessage::StartEditTask),
        Action::EditDueDate => Message::Ui(UiMessage::StartEditDueDate),
        Action::EditTags => Message::Ui(UiMessage::StartEditTags),
        Action::DeleteTask => Message::Ui(UiMessage::ShowDeleteConfirm),
        Action::CyclePriority => Message::Task(TaskMessage::CyclePriority),
        Action::ToggleTimeTracking => Message::Time(TimeMessage::ToggleTracking),
        Action::ToggleSidebar => Message::Ui(UiMessage::ToggleSidebar),
        Action::ToggleShowCompleted => Message::Ui(UiMessage::ToggleShowCompleted),
        Action::ShowHelp => Message::Ui(UiMessage::ShowHelp),
        Action::FocusSidebar => Message::Navigation(NavigationMessage::FocusSidebar),
        Action::FocusTaskList => Message::Navigation(NavigationMessage::FocusTaskList),
        Action::Select => Message::Navigation(NavigationMessage::SelectSidebarItem),
        Action::Save => Message::System(SystemMessage::Save),
        Action::Quit => Message::System(SystemMessage::Quit),
    }
}
