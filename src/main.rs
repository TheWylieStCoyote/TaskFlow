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

    // In multi-select mode, Space toggles task selection
    if model.multi_select_mode && key.code == KeyCode::Char(' ') {
        return Message::Ui(UiMessage::ToggleTaskSelection);
    }

    // In calendar view, arrow keys navigate days/weeks
    if model.current_view == taskflow::app::ViewId::Calendar
        && model.focus_pane == taskflow::app::FocusPane::TaskList
    {
        match key.code {
            KeyCode::Left => return Message::Ui(UiMessage::CalendarPrevDay),
            KeyCode::Right => return Message::Ui(UiMessage::CalendarNextDay),
            KeyCode::Char('h') => return Message::Ui(UiMessage::CalendarPrevDay),
            KeyCode::Char('l') => return Message::Ui(UiMessage::CalendarNextDay),
            _ => {}
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
                } else {
                    // Start recording to this slot
                    return Message::Ui(UiMessage::StartRecordMacro);
                }
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
        Action::CreateSubtask => Message::Ui(UiMessage::StartCreateSubtask),
        Action::CreateProject => Message::Ui(UiMessage::StartCreateProject),
        Action::EditTask => Message::Ui(UiMessage::StartEditTask),
        Action::EditDueDate => Message::Ui(UiMessage::StartEditDueDate),
        Action::EditTags => Message::Ui(UiMessage::StartEditTags),
        Action::EditDescription => Message::Ui(UiMessage::StartEditDescription),
        Action::DeleteTask => Message::Ui(UiMessage::ShowDeleteConfirm),
        Action::CyclePriority => Message::Task(TaskMessage::CyclePriority),
        Action::MoveToProject => Message::Ui(UiMessage::StartMoveToProject),
        Action::ToggleTimeTracking => Message::Time(TimeMessage::ToggleTracking),
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
    }
}
