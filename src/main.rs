use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use taskflow::app::{update, Message, Model, NavigationMessage, RunningState, SystemMessage, TaskMessage, UiMessage};
use taskflow::ui::view;

fn main() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state with sample data
    let mut model = Model::new().with_sample_data();

    // Run the app
    let result = run_app(&mut terminal, &mut model);

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
    // If help is showing, any key closes it
    if model.show_help {
        return Message::Ui(UiMessage::HideHelp);
    }

    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => Message::System(SystemMessage::Quit),

        // Help
        (KeyCode::Char('?'), _) => Message::Ui(UiMessage::ShowHelp),

        // Navigation
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
            Message::Navigation(NavigationMessage::Down)
        }
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
            Message::Navigation(NavigationMessage::Up)
        }
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

        // UI toggles
        (KeyCode::Char('c'), _) => Message::Ui(UiMessage::ToggleShowCompleted),

        _ => Message::None,
    }
}
