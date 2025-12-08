//! TUI application runner.

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::{debug, info, warn};

use taskflow::app::{update, Message, Model, PomodoroMessage, RunningState, SystemMessage};
use taskflow::config::{Keybindings, Settings, Theme};
use taskflow::storage::BackendType;
use taskflow::ui::view;

use crate::cli::Cli;
use crate::input::handle_key_event;

/// Run the TUI application
pub fn run_tui(cli: Cli) -> anyhow::Result<()> {
    // Load settings from config file
    let settings = Settings::load();
    debug!("Settings loaded from config file");

    // CLI args override config file settings
    let backend_type = if cli.backend == BackendType::Json {
        BackendType::parse(&settings.backend).unwrap_or_default()
    } else {
        cli.backend
    };

    // Determine data path (CLI > config > default)
    let data_path = cli.data.unwrap_or_else(|| settings.get_data_path());
    info!(backend = ?backend_type, path = %data_path.display(), "Initializing storage");

    // Create app state
    let mut model = if cli.demo {
        debug!("Using demo/sample data");
        Model::new().with_sample_data()
    } else {
        // Try to load from storage, fall back to sample data on error
        match Model::new().with_storage(backend_type, data_path.clone()) {
            Ok(m) => {
                if m.tasks.is_empty() {
                    debug!("No tasks found, loading sample data");
                    // No tasks loaded, use sample data
                    m.with_sample_data()
                } else {
                    info!(
                        task_count = m.tasks.len(),
                        project_count = m.projects.len(),
                        "Data loaded successfully"
                    );
                    m
                }
            }
            Err(e) => {
                // Set error state so TUI can show alert to user
                warn!(error = %e, path = %data_path.display(), "Failed to load data");
                let error_msg = format!("{}: {e}", data_path.display());
                let mut m = Model::new().with_sample_data();
                m.storage_load_error = Some(error_msg);
                m.show_storage_error_alert = true;
                m
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
        debug!("Saving data before exit");
        if let Err(e) = model.save() {
            warn!(error = %e, "Could not save data on exit");
            eprintln!("Warning: Could not save data: {e}");
        } else {
            info!("Data saved successfully");
        }
    }

    info!("TaskFlow shutting down");

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
                if let Err(e) = model.save() {
                    model.error_message = Some(format!("Auto-save failed: {e}"));
                }
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
            let playback_messages =
                if let Message::Ui(taskflow::app::UiMessage::PlayMacro(slot)) = &message {
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
