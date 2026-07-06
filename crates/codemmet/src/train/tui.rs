//! The ratatui full-screen front end.
//!
//! This layer is deliberately thin: it draws the [`Trainer`]'s state and
//! translates key presses into [`trainer events`](TrEvent). All grading,
//! progression, and hint logic lives in the state machine, so the TUI holds no
//! behavior worth unit-testing — it is verified by running it.

use std::io;

use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event::{self, Event as CEvent, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Paragraph, Wrap};

use super::exercise::{Direction, Exercise};
use super::trainer::{Event as TrEvent, Feedback, Mode, Phase, Trainer};

/// Run the trainer in a ratatui full-screen session, restoring the terminal on
/// exit (including on panic — [`ratatui::init`] installs a restoring hook).
pub fn run(trainer: &mut Trainer) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(trainer, &mut terminal);
    ratatui::restore();
    result
}

fn event_loop(trainer: &mut Trainer, terminal: &mut DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, trainer))?;
        let CEvent::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        // The done screen is dismissed by any key.
        if trainer.is_done() {
            break;
        }
        if apply_key(trainer, key.code, key.modifiers) {
            break;
        }
    }
    Ok(())
}

/// Translate a key into a trainer event. Returns `true` when the session should
/// exit immediately (a quit key).
fn apply_key(trainer: &mut Trainer, code: KeyCode, mods: KeyModifiers) -> bool {
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    if code == KeyCode::Esc || (ctrl && code == KeyCode::Char('c')) {
        trainer.handle(TrEvent::Quit);
        return true;
    }
    match trainer.phase().clone() {
        Phase::Intro(_) if code == KeyCode::Enter => trainer.handle(TrEvent::Advance),
        Phase::Feedback(_) if matches!(code, KeyCode::Enter | KeyCode::Char(' ')) => {
            trainer.handle(TrEvent::Advance);
        }
        Phase::Answering => match code {
            KeyCode::Enter => trainer.handle(TrEvent::Submit),
            KeyCode::Backspace => trainer.handle(TrEvent::Backspace),
            KeyCode::Tab => trainer.handle(TrEvent::Hint),
            KeyCode::Char('r') if ctrl => trainer.handle(TrEvent::Reveal),
            KeyCode::Char('k') if ctrl => trainer.handle(TrEvent::Skip),
            KeyCode::Char(c) if !ctrl && !c.is_control() => trainer.handle(TrEvent::Insert(c)),
            _ => {}
        },
        Phase::Sandbox => match code {
            KeyCode::Enter => trainer.handle(TrEvent::Submit),
            KeyCode::Backspace => trainer.handle(TrEvent::Backspace),
            KeyCode::Char(c) if !ctrl && !c.is_control() => trainer.handle(TrEvent::Insert(c)),
            _ => {}
        },
        _ => {}
    }
    false
}

// --- rendering ---------------------------------------------------------------

fn render(frame: &mut Frame, trainer: &Trainer) {
    let [title, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    render_title(frame, title, trainer);
    render_footer(frame, footer, trainer);
    match trainer.phase() {
        Phase::Intro(_) => render_intro(frame, body, trainer),
        Phase::Answering => render_answering(frame, body, trainer),
        Phase::Feedback(feedback) => render_feedback(frame, body, feedback),
        Phase::Sandbox => render_sandbox(frame, body, trainer),
        Phase::Done => render_done(frame, body, trainer),
    }
}

fn render_title(frame: &mut Frame, area: Rect, trainer: &Trainer) {
    let stats = trainer.stats();
    let middle = if trainer.mode() == Mode::Sandbox {
        "  free play — expand anything   ".to_string()
    } else {
        format!(
            "  {} · {} · {}   ",
            trainer.mode().title(),
            trainer.current_level().title(),
            trainer.header_note()
        )
    };
    let line = Line::from(vec![
        " codemmet trainer ".bold().on_blue().white(),
        Span::raw(middle),
        Span::styled(
            format!("solved {} · streak {}", stats.solved, stats.streak),
            Style::new().dim(),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_footer(frame: &mut Frame, area: Rect, trainer: &Trainer) {
    let keys = match trainer.phase() {
        Phase::Intro(_) => "Enter: begin    Esc: quit",
        Phase::Answering => {
            "Enter: submit    Tab: hint    Ctrl+R: reveal    Ctrl+K: skip    Esc: quit"
        }
        Phase::Feedback(_) => "Enter: continue    Esc: quit",
        Phase::Sandbox => "Enter: expand    Esc: quit",
        Phase::Done => "press any key to exit",
    };
    frame.render_widget(Paragraph::new(Line::from(keys).dim()), area);
}

fn render_intro(frame: &mut Frame, area: Rect, trainer: &Trainer) {
    let level = trainer.current_level();
    let mut lines =
        vec![Line::from(format!("Level {} — {}", level.number(), level.title())).bold()];
    lines.push(Line::raw(""));
    for line in level.blurb().lines() {
        lines.push(Line::raw(line.to_string()));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from("Press Enter to begin.").green());
    let block = Block::bordered().title(" Lesson ");
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_answering(frame: &mut Frame, area: Rect, trainer: &Trainer) {
    let [question, answer, preview] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(3),
        Constraint::Length(8),
    ])
    .areas(area);

    // The prompt, choices, and any transient wrong/invalid note. Hints and the
    // live preview go in the bottom panel so long choices never hide them.
    let mut lines: Vec<Line> = Vec::new();
    if let Some(exercise) = trainer.exercise() {
        prompt_lines(exercise, &mut lines);
    }
    if let Some(message) = trainer.message() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            message.to_string(),
            Style::new().red(),
        )));
    }
    let title = format!(" {} ", trainer.header_note());
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title(title))
            .wrap(Wrap { trim: false }),
        question,
    );

    // The answer buffer with a block cursor.
    let buffer = format!("{}█", trainer.input());
    frame.render_widget(
        Paragraph::new(Line::from(buffer)).block(Block::bordered().title(" Your answer ")),
        answer,
    );

    render_info(frame, preview, trainer);
}

/// The bottom panel: revealed hints, then (for reverse) the live expansion.
fn render_info(frame: &mut Frame, area: Rect, trainer: &Trainer) {
    let mut lines: Vec<Line> = Vec::new();
    for hint in trainer.revealed_hints() {
        lines.push(Line::from(vec![
            Span::styled("hint: ", Style::new().yellow().bold()),
            Span::styled(hint.clone(), Style::new().yellow()),
        ]));
    }

    let is_reverse = trainer
        .exercise()
        .is_some_and(|e| e.direction == Direction::Reverse);
    if is_reverse {
        if let Some(body) = trainer.live_preview() {
            if !lines.is_empty() {
                lines.push(Line::raw(""));
            }
            lines.push(Line::from("expands to:").dim());
            for line in body.lines() {
                lines.push(Line::styled(format!("  {line}"), Style::new().cyan()));
            }
        } else if lines.is_empty() {
            lines.push(Line::from("type an abbreviation to preview its expansion").dim());
        }
    } else if lines.is_empty() {
        lines.push(Line::from("type your answer above, then press Enter").dim());
    }

    let title = if is_reverse {
        " Hints & live preview "
    } else {
        " Hints "
    };
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title(title))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_feedback(frame: &mut Frame, area: Rect, feedback: &Feedback) {
    let headline = if feedback.solved {
        Line::from("✓ Correct!").green().bold()
    } else {
        Line::from("✗ Revealed").red().bold()
    };
    let lines = vec![
        headline,
        Line::raw(""),
        Line::from(vec![
            Span::styled("a canonical answer: ", Style::new().dim()),
            Span::raw(feedback.answer.clone()),
        ]),
        Line::raw(""),
        Line::from("Press Enter to continue.").green(),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title(" Result "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_done(frame: &mut Frame, area: Rect, trainer: &Trainer) {
    let stats = trainer.stats();
    // Review mode with nothing shown means the queue was empty.
    if trainer.mode() == Mode::Review && stats.presented == 0 {
        let lines = vec![
            Line::from("No reviews due 🎉").green().bold(),
            Line::raw(""),
            Line::raw("Nothing is scheduled right now — come back later,"),
            Line::raw("or run lessons or drills to build up your schedule."),
            Line::raw(""),
            Line::from("Press any key to exit.").dim(),
        ];
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .block(Block::bordered().title(" Review "))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    let accuracy = if stats.attempts == 0 {
        0
    } else {
        (stats.solved * 100) / stats.attempts
    };
    let headline = if trainer.mode() == Mode::Lessons {
        "Lessons complete!"
    } else {
        "Session complete!"
    };
    let lines = vec![
        Line::from(headline).green().bold(),
        Line::raw(""),
        Line::raw(format!("Exercises shown:  {}", stats.presented)),
        Line::raw(format!("Solved:           {}", stats.solved)),
        Line::raw(format!("Answer accuracy:  {accuracy}%")),
        Line::raw(format!("Best streak:      {}", stats.best_streak)),
        Line::raw(""),
        Line::from("Press any key to exit.").dim(),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title(" Session summary "))
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// Free-play sandbox: a scrollback of past expansions, the input, and a live
/// preview of the current buffer.
fn render_sandbox(frame: &mut Frame, area: Rect, trainer: &Trainer) {
    let [log, input, preview] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(3),
        Constraint::Length(8),
    ])
    .areas(area);

    let mut lines: Vec<Line> = Vec::new();
    for (abbr, expansion) in trainer.sandbox_log() {
        lines.push(Line::from(vec![
            Span::styled(format!("{abbr} "), Style::new().bold()),
            Span::styled("⟶", Style::new().dim()),
        ]));
        for line in expansion.lines() {
            lines.push(Line::styled(format!("  {line}"), Style::new().cyan()));
        }
        lines.push(Line::raw(""));
    }
    if let Some(message) = trainer.message() {
        lines.push(Line::from(Span::styled(
            message.to_string(),
            Style::new().red(),
        )));
    }
    if lines.is_empty() {
        lines.push(Line::from("Type an abbreviation and press Enter to expand it.").dim());
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::bordered().title(" Sandbox "))
            .wrap(Wrap { trim: false }),
        log,
    );

    let buffer = format!("{}█", trainer.input());
    frame.render_widget(
        Paragraph::new(Line::from(buffer)).block(Block::bordered().title(" Abbreviation ")),
        input,
    );

    let text = trainer.live_preview().map_or_else(
        || Text::from(Line::from("preview appears as you type").dim()),
        Text::from,
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::bordered().title(" Live preview "))
            .wrap(Wrap { trim: false }),
        preview,
    );
}

/// Push an exercise's prompt (and choices, for multiple choice) as lines.
fn prompt_lines(exercise: &Exercise, lines: &mut Vec<Line<'static>>) {
    match exercise.direction {
        Direction::Reverse => {
            lines.push(Line::from("Write an abbreviation that produces this code:").bold());
            lines.push(Line::raw(""));
            for line in exercise.target.lines() {
                lines.push(Line::styled(format!("    {line}"), Style::new().cyan()));
            }
        }
        Direction::Forward if exercise.is_multiple_choice() => {
            lines.push(
                Line::from(format!(
                    "Which output does `{}` produce?",
                    exercise.abbreviation
                ))
                .bold(),
            );
            for (i, choice) in exercise.choices.iter().enumerate() {
                lines.push(Line::raw(""));
                lines.push(Line::from(format!("  {}.", i + 1)).bold());
                for line in choice.lines() {
                    lines.push(Line::styled(format!("    {line}"), Style::new().cyan()));
                }
            }
        }
        Direction::Forward => {
            lines.push(
                Line::from(format!("What does `{}` expand to?", exercise.abbreviation)).bold(),
            );
            lines.push(Line::from("Type the code it produces.").dim());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::train::exercise::Level;
    use crate::train::review::ReviewState;
    use crate::train::trainer::Config;
    use codemmet_core::engine::Engine;
    use codemmet_core::lang::registry::Registry;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn engine() -> Engine {
        Engine::with_registry(Registry::with_builtins().expect("builtins should load"))
    }

    fn trainer(engine: &Engine) -> Trainer<'_> {
        let config = Config {
            language: "rust".to_string(),
            mode: Mode::Lessons,
            level: Some(Level::Nesting),
            seed: 5,
            now: 1_000_000,
        };
        Trainer::new(engine, config, ReviewState::default())
    }

    fn sandbox(engine: &Engine) -> Trainer<'_> {
        let config = Config {
            language: "rust".to_string(),
            mode: Mode::Sandbox,
            level: None,
            seed: 1,
            now: 0,
        };
        Trainer::new(engine, config, ReviewState::default())
    }

    /// Render one frame to an in-memory backend and return its text.
    fn draw(trainer: &Trainer) -> String {
        let mut terminal = Terminal::new(TestBackend::new(90, 30)).expect("terminal");
        terminal.draw(|frame| render(frame, trainer)).expect("draw");
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn every_phase_renders_without_panicking() {
        let engine = engine();
        let mut t = trainer(&engine);

        assert!(draw(&t).contains("codemmet trainer"));
        assert!(draw(&t).contains("Lesson"), "intro screen");

        t.handle(TrEvent::Advance);
        let answering = draw(&t);
        assert!(answering.contains("Your answer"), "answer box");
        assert!(answering.contains("Hints"), "hints/preview panel");

        t.handle(TrEvent::Reveal);
        assert!(draw(&t).contains("Result"), "feedback screen");

        // Drive to the end and render the summary.
        let mut guard = 0;
        while !t.is_done() {
            guard += 1;
            assert!(guard < 100);
            match t.phase() {
                Phase::Answering => t.handle(TrEvent::Reveal),
                _ => t.handle(TrEvent::Advance),
            }
        }
        assert!(draw(&t).contains("complete"), "done screen");
    }

    #[test]
    fn keys_map_to_events_per_phase() {
        let engine = engine();
        let mut t = trainer(&engine);

        // Intro: Enter begins.
        assert!(!apply_key(&mut t, KeyCode::Enter, KeyModifiers::empty()));
        assert_eq!(t.phase(), &Phase::Answering);

        // Answering: characters build the buffer, Backspace edits it.
        apply_key(&mut t, KeyCode::Char('p'), KeyModifiers::empty());
        apply_key(&mut t, KeyCode::Char('r'), KeyModifiers::empty());
        assert_eq!(t.input(), "pr");
        apply_key(&mut t, KeyCode::Backspace, KeyModifiers::empty());
        assert_eq!(t.input(), "p");

        // Tab reveals a hint.
        apply_key(&mut t, KeyCode::Tab, KeyModifiers::empty());
        assert_eq!(t.revealed_hints().len(), 1);

        // Ctrl+R reveals the answer and moves to feedback.
        apply_key(&mut t, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert!(matches!(t.phase(), Phase::Feedback(_)));
    }

    #[test]
    fn escape_requests_quit() {
        let engine = engine();
        let mut t = trainer(&engine);
        assert!(apply_key(&mut t, KeyCode::Esc, KeyModifiers::empty()));
        assert!(t.is_done());
    }

    #[test]
    fn sandbox_renders_and_expands_input() {
        let engine = engine();
        let mut t = sandbox(&engine);
        assert!(draw(&t).contains("Sandbox"));
        for c in "pr".chars() {
            apply_key(&mut t, KeyCode::Char(c), KeyModifiers::empty());
        }
        // Live preview shows the expansion before submitting.
        assert!(draw(&t).contains("println!"));
        apply_key(&mut t, KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(t.sandbox_log().len(), 1);
    }
}
