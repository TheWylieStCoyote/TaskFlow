//! The line-based front end: a read-eval-print loop over the [`Trainer`] state
//! machine. It is the non-TTY / piped fallback for the ratatui TUI, and reads
//! from any [`BufRead`] / writes any [`Write`] so it can be driven headlessly
//! in tests.

use std::io::{BufRead, Write};

use super::exercise::Direction;
use super::trainer::{Event, Feedback, Phase, Trainer};

/// Run the line loop until the trainer is [`Phase::Done`] or input ends.
pub fn run_plain<R: BufRead, W: Write>(
    trainer: &mut Trainer,
    input: &mut R,
    out: &mut W,
) -> std::io::Result<()> {
    writeln!(
        out,
        "codemmet trainer — answer, or :hint  :show  :skip  :quit"
    )?;
    loop {
        match trainer.phase().clone() {
            Phase::Intro(level) => {
                writeln!(out)?;
                writeln!(out, "── level {} · {} ──", level.number(), level.title())?;
                for line in level.blurb().lines() {
                    writeln!(out, "  {line}")?;
                }
                write!(out, "[enter] begin > ")?;
                out.flush()?;
                advance_or_quit(trainer, input)?;
            }
            Phase::Answering => {
                answering_screen(trainer, out)?;
                write!(out, "{} ", caret(trainer))?;
                out.flush()?;
                let Some(line) = read_line(input)? else {
                    trainer.handle(Event::Quit);
                    continue;
                };
                dispatch(trainer, line.trim(), out)?;
            }
            Phase::Feedback(feedback) => {
                feedback_screen(&feedback, out)?;
                write!(out, "[enter] continue > ")?;
                out.flush()?;
                advance_or_quit(trainer, input)?;
            }
            Phase::Done => break,
        }
    }
    Ok(())
}

/// At an intro/feedback screen: any line advances, `:quit` or EOF ends.
fn advance_or_quit<R: BufRead>(trainer: &mut Trainer, input: &mut R) -> std::io::Result<()> {
    match read_line(input)? {
        Some(line) if !is_quit(line.trim()) => trainer.handle(Event::Advance),
        _ => trainer.handle(Event::Quit),
    }
    Ok(())
}

/// Map an answer-line to an event.
fn dispatch<W: Write>(trainer: &mut Trainer, line: &str, out: &mut W) -> std::io::Result<()> {
    match line {
        ":quit" | ":q" => trainer.handle(Event::Quit),
        ":hint" => trainer.handle(Event::Hint),
        ":show" => trainer.handle(Event::Reveal),
        ":skip" | ":next" => trainer.handle(Event::Skip),
        ":help" | ":?" => writeln!(out, "  commands: :hint  :show  :skip  :quit")?,
        answer => trainer.submit_answer(answer),
    }
    Ok(())
}

/// The prompt, any revealed hints, a transient note, and the live preview.
fn answering_screen<W: Write>(trainer: &Trainer, out: &mut W) -> std::io::Result<()> {
    prompt(trainer, out)?;
    for hint in trainer.revealed_hints() {
        writeln!(out, "  hint: {hint}")?;
    }
    if let Some(message) = trainer.message() {
        writeln!(out, "  {message}")?;
    }
    if let Some(preview) = trainer.live_preview() {
        writeln!(out, "  ⟶ that currently expands to:")?;
        indented(&preview, out)?;
    }
    Ok(())
}

fn prompt<W: Write>(trainer: &Trainer, out: &mut W) -> std::io::Result<()> {
    let Some(exercise) = trainer.exercise() else {
        return Ok(());
    };
    let level = trainer.current_level();
    let (done, total) = trainer.level_progress();
    writeln!(
        out,
        "── level {} · {} — exercise {}/{total} ──",
        level.number(),
        level.title(),
        done + 1,
    )?;
    match exercise.direction {
        Direction::Reverse => {
            writeln!(out, "Write an abbreviation that produces this code:")?;
            indented(&exercise.target, out)
        }
        Direction::Forward if exercise.is_multiple_choice() => {
            writeln!(
                out,
                "Which output does `{}` produce?",
                exercise.abbreviation
            )?;
            for (i, choice) in exercise.choices.iter().enumerate() {
                writeln!(out, "  {}.", i + 1)?;
                indented(choice, out)?;
            }
            Ok(())
        }
        Direction::Forward => writeln!(
            out,
            "What does `{}` expand to? (type the code)",
            exercise.abbreviation
        ),
    }
}

fn feedback_screen<W: Write>(feedback: &Feedback, out: &mut W) -> std::io::Result<()> {
    if feedback.solved {
        writeln!(out, "  ✓ correct!")?;
    } else if feedback.revealed {
        writeln!(out, "  ✗ the answer was revealed")?;
    }
    writeln!(out, "  answer: {}", feedback.answer)
}

/// The input caret for the current exercise.
fn caret(trainer: &Trainer) -> &'static str {
    match trainer.exercise().map(|e| e.direction) {
        Some(Direction::Forward) => "answer>",
        _ => "abbr>",
    }
}

fn is_quit(line: &str) -> bool {
    matches!(line, ":quit" | ":q")
}

/// Read a line, returning `None` at end of input.
fn read_line<R: BufRead>(input: &mut R) -> std::io::Result<Option<String>> {
    let mut line = String::new();
    if input.read_line(&mut line)? == 0 {
        Ok(None)
    } else {
        Ok(Some(line))
    }
}

/// Write `text` with every line indented, for readable code blocks.
fn indented<W: Write>(text: &str, out: &mut W) -> std::io::Result<()> {
    for line in text.lines() {
        writeln!(out, "    {line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::train::exercise::{Exercise, Level, generate};
    use crate::train::rng::Rng;
    use codemmet_core::context::{ExpansionContext, IndentUnit};
    use codemmet_core::engine::Engine;
    use codemmet_core::lang::registry::Registry;
    use std::io::Cursor;

    fn engine() -> Engine {
        Engine::with_registry(Registry::with_builtins().expect("builtins should load"))
    }

    fn ctx() -> ExpansionContext {
        ExpansionContext::plain(String::new(), IndentUnit::Spaces(4))
    }

    fn solution(exercise: &Exercise) -> String {
        match exercise.direction {
            Direction::Reverse => exercise.abbreviation.clone(),
            Direction::Forward if exercise.is_multiple_choice() => {
                (exercise.answer_index + 1).to_string()
            }
            Direction::Forward => exercise.target.clone(),
        }
    }

    fn drive(trainer: &mut Trainer, script: &str) -> String {
        let mut input = Cursor::new(script.to_string());
        let mut out: Vec<u8> = Vec::new();
        run_plain(trainer, &mut input, &mut out).expect("plain runner");
        String::from_utf8(out).expect("utf8 output")
    }

    #[test]
    fn quitting_at_the_intro_ends_cleanly() {
        let engine = engine();
        let mut trainer = Trainer::new(&engine, "rust", ctx(), Some(Level::SingleTrigger), 1);
        let text = drive(&mut trainer, ":quit\n");
        assert!(text.contains("codemmet trainer"));
        assert!(trainer.is_done());
        assert_eq!(trainer.stats().solved, 0);
    }

    #[test]
    fn a_correct_answer_is_accepted() {
        let engine = engine();
        let seed = 7;
        let mut probe = Rng::new(seed);
        let exercise =
            generate(&engine, "rust", Level::Nesting, &ctx(), &mut probe).expect("generated");
        let answer = solution(&exercise);

        let mut trainer = Trainer::new(&engine, "rust", ctx(), Some(Level::Nesting), seed);
        // Enter to begin, then the answer; EOF then stops at the feedback screen.
        let text = drive(&mut trainer, &format!("\n{answer}\n"));
        assert!(text.contains("✓ correct"), "output:\n{text}");
        assert_eq!(trainer.stats().solved, 1);
    }

    #[test]
    fn hint_then_show_reveals_the_answer() {
        let engine = engine();
        let mut trainer = Trainer::new(&engine, "rust", ctx(), Some(Level::Nesting), 3);
        let text = drive(&mut trainer, "\n:hint\n:show\n");
        assert!(text.contains("hint:"), "output:\n{text}");
        assert!(text.contains("answer:"), "output:\n{text}");
        assert_eq!(trainer.stats().solved, 0);
    }
}
