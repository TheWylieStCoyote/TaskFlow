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
    update, Message, Model, NavigationMessage, RunningState, SystemMessage, TaskMessage, TimeMessage, UiMessage,
};
use taskflow::config::Settings;
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
    let data_path = args.data.clone().unwrap_or_else(|| settings.get_data_path());

    // Parse backend type
    let backend_type = BackendType::from_str(backend_str).unwrap_or_default();

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
    model.refresh_visible_tasks();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal, &mut model);

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
) -> anyhow::Result<()> {
    loop {
        // Draw UI
        terminal.draw(|frame| view(model, frame))?;

        // Check if quitting
        if model.running == RunningState::Quitting {
            break;
        }

        // Handle events with timeout for potential async operations
        if event::poll(Duration::from_millis(100))? {
            let message = match event::read()? {
                Event::Key(key) => handle_key_event(key, model),
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

fn handle_key_event(key: event::KeyEvent, model: &Model) -> Message {
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

    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => Message::System(SystemMessage::Quit),

        // Help
        (KeyCode::Char('?'), _) => Message::Ui(UiMessage::ShowHelp),

        // Save
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => Message::System(SystemMessage::Save),

        // Navigation
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
            Message::Navigation(NavigationMessage::Down)
        }
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => Message::Navigation(NavigationMessage::Up),
        (KeyCode::Char('g'), _) => Message::Navigation(NavigationMessage::First),
        (KeyCode::Char('G'), _) => Message::Navigation(NavigationMessage::Last),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) | (KeyCode::PageUp, _) => {
            Message::Navigation(NavigationMessage::PageUp)
        }
        (KeyCode::Char('d'), KeyModifiers::CONTROL) | (KeyCode::PageDown, _) => {
            Message::Navigation(NavigationMessage::PageDown)
        }

        // Task actions
        (KeyCode::Char('x'), _) | (KeyCode::Char(' '), _) => {
            Message::Task(TaskMessage::ToggleComplete)
        }
        (KeyCode::Char('a'), _) => Message::Ui(UiMessage::StartCreateTask),
        (KeyCode::Char('D'), _) => Message::Ui(UiMessage::ShowDeleteConfirm),

        // Time tracking
        (KeyCode::Char('t'), _) => Message::Time(TimeMessage::ToggleTracking),

        // UI toggles
        (KeyCode::Char('c'), _) => Message::Ui(UiMessage::ToggleShowCompleted),
        (KeyCode::Char('b'), _) => Message::Ui(UiMessage::ToggleSidebar),

        _ => Message::None,
    }
}
