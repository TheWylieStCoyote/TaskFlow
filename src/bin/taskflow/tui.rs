//! TUI application runner.

use std::io::{self, Write as _};
#[cfg(unix)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::{debug, error, info, warn};

use taskflow::app::{
    update, Message, Model, PomodoroMessage, RunningState, SystemMessage, TaskTemplate,
};
use taskflow::config::{FirstRunMode, Keybindings, Settings, Theme};
use taskflow::domain::Priority;
use taskflow::storage::BackendType;
use taskflow::ui::view;

use crate::cli::Cli;
use crate::input::handle_key_event;

/// Run the TUI application
pub fn run_tui(cli: Cli) -> anyhow::Result<()> {
    // Check if config file exists before loading (for later prompt)
    let config_existed = Settings::config_exists();

    // Load settings from config file (uses defaults if missing)
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
        // Try to load from storage
        match Model::new().with_storage(backend_type, data_path.clone()) {
            Ok(m) => {
                if m.tasks.is_empty() {
                    // Check first_run_mode setting to decide what to do
                    match settings.first_run_mode {
                        FirstRunMode::Demo => {
                            debug!("No tasks found, loading sample data (first_run_mode=demo)");
                            m.with_sample_data()
                        }
                        FirstRunMode::Empty => {
                            info!("No tasks found, starting with empty database (first_run_mode=empty)");
                            m
                        }
                    }
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
                // On error, respect first_run_mode setting
                match settings.first_run_mode {
                    FirstRunMode::Demo => {
                        let mut m = Model::new().with_sample_data();
                        m.alerts.storage_error = Some(error_msg);
                        m.alerts.show_storage_error = true;
                        m
                    }
                    FirstRunMode::Empty => {
                        let mut m = Model::new();
                        m.alerts.storage_error = Some(error_msg);
                        m.alerts.show_storage_error = true;
                        m
                    }
                }
            }
        }
    };

    // Apply settings to model
    model.show_sidebar = settings.show_sidebar;
    model.filtering.show_completed = settings.show_completed;
    model.default_priority = settings.default_priority();

    // Load user-defined templates from config
    for cfg in &settings.custom_templates {
        let mut template = TaskTemplate::new(&cfg.name, &cfg.title)
            .with_priority(Priority::parse(&cfg.priority).unwrap_or_default())
            .with_tags(cfg.tags.clone());
        if let Some(ref desc) = cfg.description {
            template = template.with_description(desc);
        }
        if let Some(days) = cfg.due_days {
            template = template.with_due_days(days);
        }
        model.template_manager.add_template(template);
    }

    model.refresh_visible_tasks();

    // Check for overdue tasks and show alert at startup
    model.check_overdue_alert();

    // Desktop notifications for overdue and due-today tasks at startup
    {
        let (overdue_count, _) = model.overdue_summary();
        taskflow::notifications::notify_overdue_tasks(overdue_count);
        let due_today = model.footer_stats.due_today_count;
        taskflow::notifications::notify_due_today(due_today);
    }

    // Prompt to generate config file if it didn't exist at startup
    if !config_existed {
        model.show_generate_config_prompt = true;
    }

    // Load keybindings and theme
    let keybindings = Keybindings::load();
    let theme = Theme::load(&settings.theme);

    // Start editor socket server on Unix when enabled
    #[cfg(unix)]
    let (editor_server, socket_state) = {
        use std::sync::{mpsc, Arc, Mutex};
        use taskflow::socket;
        use taskflow::socket::types::{EditorWriteRequest, ModelSnapshot};

        if settings.editor.enabled {
            let snapshot_arc = Arc::new(Mutex::new(ModelSnapshot::from_model(&model)));
            let (write_tx, write_rx) = mpsc::sync_channel::<EditorWriteRequest>(64);
            match socket::start(&settings, write_tx, Arc::clone(&snapshot_arc)) {
                Ok(server) => (Some(server), Some((write_rx, snapshot_arc))),
                Err(e) => {
                    // Keep running without the editor socket, but say so where
                    // the user will see it instead of only in the debug log.
                    model.alerts.error_message = Some(format!("Editor socket disabled: {e}"));
                    (None, None)
                }
            }
        } else {
            (None, None)
        }
    };

    // Termination signals (an editor closing its terminal sends SIGHUP) ask
    // the loop to quit, so the shutdown below still saves and cleans up.
    #[cfg(unix)]
    let shutdown = register_shutdown_signals();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(
        &mut terminal,
        &mut model,
        &keybindings,
        &settings,
        &theme,
        #[cfg(unix)]
        socket_state.as_ref(),
        #[cfg(unix)]
        &shutdown,
    );

    // Save before exit if storage is configured and not in sample data mode.
    // This runs whether `run_app` returned Ok or Err (for example a failed
    // draw after the controlling terminal went away).
    if model.has_storage() && !model.storage.sample_data_mode && model.storage.dirty {
        debug!("Saving data before exit");
        if let Err(e) = model.save() {
            warn!(error = %e, "Could not save data on exit");
            // `eprintln!` panics when stderr is gone (after SIGHUP), so
            // write and ignore failures.
            let _ = writeln!(
                io::stderr(),
                "Warning: Could not save data on exit: {e}\n  Your changes may be lost. Check disk space and permissions."
            );
        } else {
            info!("Data saved successfully");
        }
    }

    // Remove the editor socket before touching the terminal, so a failed
    // terminal restore cannot leave a stale socket file behind.
    #[cfg(unix)]
    drop(editor_server);

    info!("TaskFlow shutting down");

    let restored = restore_terminal(&mut terminal);

    if let Err(err) = result {
        error!(error = ?err, "TUI error");
        let _ = writeln!(io::stderr(), "Error: {err:?}");
    }

    // After a hangup the terminal is gone and restoring it is expected to
    // fail; that is not an error worth a non-zero exit.
    #[cfg(unix)]
    if shutdown.load(Ordering::Relaxed) {
        if let Err(e) = restored {
            debug!(error = %e, "Terminal restore failed after termination signal");
        }
        return Ok(());
    }
    restored
}

/// Leave raw mode and the alternate screen and show the cursor again.
///
/// Every step is attempted even if an earlier one fails; the first error is
/// returned.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let raw = disable_raw_mode();
    let screen = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let cursor = terminal.show_cursor();
    raw?;
    screen?;
    cursor?;
    Ok(())
}

/// Wait up to `timeout` for a terminal event, like [`event::poll`].
///
/// crossterm cannot cope with a hung-up terminal: its read loop only stops on
/// `WouldBlock`, so once the terminal is gone (the editor closed it) it spins
/// forever and the event loop never sees the shutdown flag. When stdin is the
/// terminal, it is polled here first, so that a hangup becomes an error that
/// ends the event loop (the shutdown still saves) and a termination signal
/// interrupts the wait at once.
#[cfg(unix)]
fn poll_terminal_event(timeout: Duration, shutdown: &AtomicBool) -> io::Result<bool> {
    use rustix::event::{poll, PollFd, PollFlags};
    use std::os::fd::AsFd;

    let stdin = io::stdin();
    if !rustix::termios::isatty(stdin.as_fd()) {
        // crossterm opened /dev/tty itself; there is nothing to watch here.
        return event::poll(timeout);
    }

    // Wait for stdin; `Ok(())` on input or interruption, an error on hangup.
    let wait = |timeout_ms: i32| -> io::Result<()> {
        let mut fds = [PollFd::new(&stdin, PollFlags::IN)];
        match poll(&mut fds, timeout_ms) {
            Ok(_) | Err(rustix::io::Errno::INTR) => {}
            Err(e) => return Err(e.into()),
        }
        if fds[0]
            .revents()
            .intersects(PollFlags::HUP | PollFlags::ERR | PollFlags::NVAL)
        {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "terminal hung up",
            ));
        }
        Ok(())
    };

    // Events crossterm has already buffered (a paste, a burst of keys) must
    // not wait for the timeout, so ask crossterm first, but only after making
    // sure the terminal is still there.
    wait(0)?;
    if event::poll(Duration::ZERO)? {
        return Ok(true);
    }
    wait(i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX))?;
    if shutdown.load(Ordering::Relaxed) {
        return Ok(false);
    }
    // Input arrived, a resize signal interrupted the wait, or it timed out.
    event::poll(Duration::ZERO)
}

/// Register SIGHUP, SIGTERM and SIGINT to set the returned flag.
///
/// The flag replaces the default action (immediate termination), so the
/// event loop can notice it and run the normal shutdown: save, remove the
/// editor socket, restore the terminal. A signal that cannot be registered
/// keeps its default action and is logged.
#[cfg(unix)]
fn register_shutdown_signals() -> Arc<AtomicBool> {
    use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGTERM};

    let flag = Arc::new(AtomicBool::new(false));
    for signal in [SIGHUP, SIGTERM, SIGINT] {
        if let Err(e) = signal_hook::flag::register(signal, Arc::clone(&flag)) {
            warn!(signal, error = %e, "Could not install signal handler");
        }
    }
    flag
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    model: &mut Model,
    keybindings: &Keybindings,
    settings: &Settings,
    theme: &Theme,
    #[cfg(unix)] socket_state: Option<&(
        std::sync::mpsc::Receiver<taskflow::socket::types::EditorWriteRequest>,
        std::sync::Arc<std::sync::Mutex<taskflow::socket::types::ModelSnapshot>>,
    )>,
    #[cfg(unix)] shutdown: &AtomicBool,
) -> anyhow::Result<()> {
    use std::time::Instant;

    let auto_save_interval = if settings.auto_save_interval > 0 {
        Some(Duration::from_secs(settings.auto_save_interval))
    } else {
        None
    };
    let mut last_save = Instant::now();
    let mut last_pomodoro_tick = Instant::now();
    let mut last_git_check = Instant::now();
    let git_check_interval = Duration::from_secs(60); // Check every 60 seconds

    loop {
        // A termination signal is a quit request. Check it before drawing:
        // after SIGHUP the terminal is gone and the draw would fail.
        #[cfg(unix)]
        if shutdown.load(Ordering::Relaxed) {
            info!("Termination signal received, shutting down");
            model.running = RunningState::Quitting;
        }

        // Check if quitting
        if model.running == RunningState::Quitting {
            break;
        }

        // Draw UI. This runs on every pass, and `event::poll` below returns
        // after at most 100ms when idle, so live readouts (the footer and
        // dashboard task timer, the Pomodoro clock) refresh several times a
        // second without any extra tick message; ratatui only writes the
        // cells that changed.
        terminal.draw(|frame| view(model, frame, theme))?;

        // Auto-save if interval has passed and there are unsaved changes (skip in sample data mode)
        if let Some(interval) = auto_save_interval {
            if model.storage.dirty
                && model.has_storage()
                && !model.storage.sample_data_mode
                && last_save.elapsed() >= interval
            {
                if let Err(e) = model.save() {
                    model.alerts.error_message = Some(format!("Auto-save failed: {e}"));
                }
                last_save = Instant::now();
            }
        }

        // Tick Pomodoro timer every second if active and not paused
        if last_pomodoro_tick.elapsed() >= Duration::from_secs(1) {
            if let Some(ref session) = model.pomodoro.session {
                if !session.paused {
                    update(model, Message::Pomodoro(PomodoroMessage::Tick));
                }
            }
            last_pomodoro_tick = Instant::now();
        }

        // Check for merged git branches periodically
        if last_git_check.elapsed() >= git_check_interval {
            update(model, Message::System(SystemMessage::CheckMergedBranches));
            last_git_check = Instant::now();
        }

        // Editor socket: drain write requests, then refresh the snapshot (Unix only)
        #[cfg(unix)]
        if let Some((write_rx, snapshot_arc)) = socket_state {
            use taskflow::socket::types::ModelSnapshot;

            // Drain pending write requests from the socket thread first, so
            // the snapshot refreshed below already reflects them and a read
            // that follows a write response sees the new state.
            if drain_editor_writes(model, write_rx) {
                // The drain saved any dirty state, so the auto-save timer
                // starts over.
                last_save = Instant::now();
            }

            // Refresh snapshot so the socket thread has an up-to-date view
            if let Ok(mut snap) = snapshot_arc.lock() {
                *snap = ModelSnapshot::from_model(model);
            }
        }

        // Handle events with timeout for potential async operations
        #[cfg(unix)]
        let ready = poll_terminal_event(Duration::from_millis(100), shutdown)?;
        #[cfg(not(unix))]
        let ready = event::poll(Duration::from_millis(100))?;
        if ready {
            let message = match event::read()? {
                Event::Key(key) => handle_key_event(key, model, keybindings),
                Event::Mouse(mouse) => crate::input::handle_mouse_event(mouse, model),
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

/// Apply every editor write request waiting on `write_rx` to `model`.
///
/// Each request gets its response on its `respond_to` channel. When at least
/// one request was applied, the derived view lists are rebuilt (pipe handlers
/// mutate the model directly) and dirty storage is saved right away instead
/// of waiting for the auto-save interval, so an editor write survives the TUI
/// being killed. A failed save is shown in `model.alerts.error_message`.
///
/// Returns `true` when at least one request was applied.
#[cfg(unix)]
pub(crate) fn drain_editor_writes(
    model: &mut Model,
    write_rx: &std::sync::mpsc::Receiver<taskflow::socket::types::EditorWriteRequest>,
) -> bool {
    use crate::commands::pipe::handlers::dispatch;
    use crate::commands::pipe::types::PipeRequest;
    use taskflow::socket::types::SocketResponse;

    let mut applied_write = false;
    while let Ok(req) = write_rx.try_recv() {
        applied_write = true;
        let response = if let Some(task_id) = req.focus_task_id {
            // Navigate TUI to the specified task
            update(
                model,
                Message::System(SystemMessage::FocusTaskById(task_id)),
            );
            SocketResponse::ok(serde_json::Value::Null)
        } else {
            // Standard pipe operation: convert PipeEnvelope -> PipeRequest via JSON
            match serde_json::to_value(&req.pipe_envelope) {
                Err(e) => SocketResponse::err("SERIALIZATION_ERROR", e.to_string()),
                Ok(pipe_json) => match serde_json::from_value::<PipeRequest>(pipe_json) {
                    Ok(pipe_req) => match dispatch(model, &pipe_req) {
                        Ok(data) => SocketResponse::ok(data),
                        Err(e) => SocketResponse::err(&e.code, &e.message),
                    },
                    Err(e) => SocketResponse::err("INVALID_REQUEST", e.to_string()),
                },
            }
        };
        let _ = req.respond_to.send(response);
    }

    if applied_write {
        // Pipe handlers mutate the model directly; the view lists are
        // derived, so rebuild them or an editor-created task stays
        // invisible until the next view change.
        model.refresh_visible_tasks();

        // Persist now rather than at the next auto-save: the TUI often runs
        // hidden in an editor terminal and may be killed with it.
        if model.has_storage() && !model.storage.sample_data_mode && model.storage.dirty {
            if let Err(e) = model.save() {
                warn!(error = %e, "Saving after editor write failed");
                model.alerts.error_message = Some(format!("Save after editor write failed: {e}"));
            }
        }
    }

    applied_write
}
