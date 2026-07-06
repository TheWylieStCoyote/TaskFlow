//! The interactive line-based trainer session.
//!
//! This is the T0/T1 front end: a plain read-eval-print loop over the pure core
//! (generation, grading, hints). The T2 milestone adds a ratatui TUI on top of
//! the same core; this line mode stays as the non-TTY / piped fallback.
//!
//! The loop reads from any [`BufRead`] and writes to any [`Write`], so it can
//! be driven headlessly by a scripted input buffer in tests.

use std::io::{BufRead, Write};

use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::Engine;

use super::exercise::{Direction, Exercise, Level, generate};
use super::grade::{Outcome, grade_forward_choice, grade_forward_free, grade_reverse};
use super::hint::hints;
use super::rng::Rng;

/// Auto-reveal the answer after this many wrong attempts on one exercise.
const MAX_MISSES: u32 = 3;

/// Session-wide tally.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Stats {
    /// Exercises shown.
    pub presented: u32,
    /// Exercises solved before a reveal.
    pub solved: u32,
    /// Gradeable answers submitted (excludes unparseable input).
    pub attempts: u32,
    /// Current correct streak.
    pub streak: u32,
    /// Longest correct streak this session.
    pub best_streak: u32,
}

/// A running trainer session over the pure core.
pub struct Session<'a> {
    engine: &'a Engine,
    language: String,
    ctx: ExpansionContext,
    /// A fixed level, or `None` to mix levels each exercise.
    level: Option<Level>,
    rng: Rng,
    stats: Stats,
}

impl<'a> Session<'a> {
    /// A new session. `level` fixes the difficulty; `None` mixes all levels.
    pub fn new(
        engine: &'a Engine,
        language: impl Into<String>,
        ctx: ExpansionContext,
        level: Option<Level>,
        seed: u64,
    ) -> Self {
        Self {
            engine,
            language: language.into(),
            ctx,
            level,
            rng: Rng::new(seed),
            stats: Stats::default(),
        }
    }

    /// Run the loop until `:quit` or end of input.
    pub fn run<R: BufRead, W: Write>(&mut self, input: &mut R, out: &mut W) -> std::io::Result<()> {
        intro(out)?;
        loop {
            let level = self.level.unwrap_or_else(|| {
                self.rng
                    .choice(&Level::ALL)
                    .copied()
                    .unwrap_or(Level::SingleTrigger)
            });
            let Some(exercise) =
                generate(self.engine, &self.language, level, &self.ctx, &mut self.rng)
            else {
                writeln!(out, "No exercises available for {}.", self.language)?;
                return Ok(());
            };
            self.stats.presented += 1;
            if self.present(&exercise, input, out)?.is_quit() {
                break;
            }
            writeln!(out)?;
        }
        summary(&self.stats, out)?;
        Ok(())
    }

    /// Run one exercise's attempt loop to completion (solved, revealed, or quit).
    fn present<R: BufRead, W: Write>(
        &mut self,
        exercise: &Exercise,
        input: &mut R,
        out: &mut W,
    ) -> std::io::Result<Flow> {
        prompt(exercise, out)?;
        let hint_list = hints(exercise);
        let mut next_hint = 0;
        let mut misses = 0;
        loop {
            write!(out, "{} ", caret(exercise))?;
            out.flush()?;
            let Some(line) = read_line(input)? else {
                return Ok(Flow::Quit);
            };
            let line = line.trim();

            if let Some(rest) = line.strip_prefix(':') {
                match rest {
                    "q" | "quit" => return Ok(Flow::Quit),
                    "?" | "help" => {
                        help(out)?;
                    }
                    "hint" => {
                        if let Some(h) = hint_list.get(next_hint) {
                            writeln!(out, "  hint: {h}")?;
                            next_hint += 1;
                        } else {
                            writeln!(out, "  no more hints — :show reveals the answer")?;
                        }
                    }
                    "show" | "skip" | "next" | "n" => {
                        reveal(exercise, out)?;
                        self.stats.streak = 0;
                        return Ok(Flow::Continue);
                    }
                    other => writeln!(out, "  unknown command `:{other}` — :help for commands")?,
                }
                continue;
            }

            match self.grade(exercise, line) {
                Outcome::Correct => {
                    writeln!(out, "  ✓ correct")?;
                    self.stats.attempts += 1;
                    self.stats.solved += 1;
                    self.stats.streak += 1;
                    self.stats.best_streak = self.stats.best_streak.max(self.stats.streak);
                    return Ok(Flow::Continue);
                }
                Outcome::Wrong { rendered } => {
                    self.stats.attempts += 1;
                    self.stats.streak = 0;
                    misses += 1;
                    if let Some(body) = rendered {
                        writeln!(out, "  ✗ that expands to:")?;
                        indented(&body, out)?;
                    } else {
                        writeln!(out, "  ✗ not quite")?;
                    }
                    if misses >= MAX_MISSES {
                        reveal(exercise, out)?;
                        return Ok(Flow::Continue);
                    }
                    writeln!(out, "  try again, or :hint / :show")?;
                }
                Outcome::Invalid { message } => {
                    writeln!(out, "  ! {message}")?;
                }
            }
        }
    }

    /// Grade a raw input line against an exercise.
    fn grade(&self, exercise: &Exercise, line: &str) -> Outcome {
        match exercise.direction {
            Direction::Reverse => grade_reverse(
                self.engine,
                &self.language,
                &self.ctx,
                line,
                &exercise.target,
            ),
            Direction::Forward if exercise.is_multiple_choice() => match line.parse::<usize>() {
                Ok(n) if (1..=exercise.choices.len()).contains(&n) => {
                    grade_forward_choice(n - 1, exercise.answer_index)
                }
                _ => Outcome::Invalid {
                    message: format!("enter a number between 1 and {}", exercise.choices.len()),
                },
            },
            Direction::Forward => grade_forward_free(line, &exercise.target),
        }
    }
}

/// Whether an exercise ended in a quit request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Continue,
    Quit,
}

impl Flow {
    const fn is_quit(self) -> bool {
        matches!(self, Self::Quit)
    }
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

/// The input caret for an exercise.
const fn caret(exercise: &Exercise) -> &'static str {
    match exercise.direction {
        Direction::Reverse => "abbr>",
        Direction::Forward => "answer>",
    }
}

fn intro<W: Write>(out: &mut W) -> std::io::Result<()> {
    writeln!(out, "codemmet trainer — :help for commands, :quit to stop")?;
    writeln!(out)
}

fn help<W: Write>(out: &mut W) -> std::io::Result<()> {
    writeln!(out, "  commands: :hint  :show  :skip  :help  :quit")
}

fn prompt<W: Write>(exercise: &Exercise, out: &mut W) -> std::io::Result<()> {
    writeln!(
        out,
        "── level {} · {} ──",
        exercise.level.number(),
        exercise.level.title()
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

fn reveal<W: Write>(exercise: &Exercise, out: &mut W) -> std::io::Result<()> {
    match exercise.direction {
        Direction::Reverse => writeln!(out, "  answer: {}", exercise.abbreviation),
        Direction::Forward if exercise.is_multiple_choice() => {
            writeln!(out, "  answer: option {}", exercise.answer_index + 1)
        }
        Direction::Forward => {
            writeln!(out, "  answer:")?;
            indented(&exercise.target, out)
        }
    }
}

fn summary<W: Write>(stats: &Stats, out: &mut W) -> std::io::Result<()> {
    let accuracy = if stats.attempts == 0 {
        0
    } else {
        (stats.solved * 100) / stats.attempts
    };
    writeln!(
        out,
        "session: {} solved of {} shown · {accuracy}% of answers correct · best streak {}",
        stats.solved, stats.presented, stats.best_streak
    )
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
    use codemmet_core::context::IndentUnit;
    use codemmet_core::lang::registry::Registry;
    use std::io::Cursor;

    fn engine() -> Engine {
        Engine::with_registry(Registry::with_builtins().expect("builtins should load"))
    }

    fn ctx() -> ExpansionContext {
        ExpansionContext::plain(String::new(), IndentUnit::Spaces(4))
    }

    /// The answer that solves an exercise, for the given input format.
    fn correct_answer(exercise: &Exercise) -> String {
        match exercise.direction {
            Direction::Reverse => exercise.abbreviation.clone(),
            Direction::Forward if exercise.is_multiple_choice() => {
                (exercise.answer_index + 1).to_string()
            }
            Direction::Forward => exercise.target.clone(),
        }
    }

    #[test]
    fn quitting_immediately_prints_a_summary() {
        let engine = engine();
        let mut session = Session::new(&engine, "rust", ctx(), Some(Level::SingleTrigger), 1);
        let mut input = Cursor::new(":quit\n");
        let mut out: Vec<u8> = Vec::new();
        session.run(&mut input, &mut out).expect("session runs");
        let text = String::from_utf8(out).expect("utf8 output");
        assert!(text.contains("session:"));
        assert_eq!(session.stats.solved, 0);
    }

    #[test]
    fn a_correct_answer_is_accepted_and_counted() {
        let engine = engine();
        let seed = 7;
        let level = Level::Nesting;

        // Replicate the first generated exercise so we know its answer.
        let mut probe = Rng::new(seed);
        let exercise =
            generate(&engine, "rust", level, &ctx(), &mut probe).expect("exercise generated");
        let answer = correct_answer(&exercise);

        let mut session = Session::new(&engine, "rust", ctx(), Some(level), seed);
        let mut input = Cursor::new(format!("{answer}\n:quit\n"));
        let mut out: Vec<u8> = Vec::new();
        session.run(&mut input, &mut out).expect("session runs");

        let text = String::from_utf8(out).expect("utf8 output");
        assert!(text.contains("✓ correct"), "output was:\n{text}");
        assert_eq!(session.stats.solved, 1);
        assert_eq!(session.stats.best_streak, 1);
    }

    #[test]
    fn hint_then_show_reveals_without_solving() {
        let engine = engine();
        let mut session = Session::new(&engine, "rust", ctx(), Some(Level::Nesting), 3);
        let mut input = Cursor::new(":hint\n:show\n:quit\n");
        let mut out: Vec<u8> = Vec::new();
        session.run(&mut input, &mut out).expect("session runs");
        let text = String::from_utf8(out).expect("utf8 output");
        assert!(text.contains("hint:"), "output was:\n{text}");
        assert!(text.contains("answer:"), "output was:\n{text}");
        assert_eq!(session.stats.solved, 0);
    }

    #[test]
    fn a_wrong_reverse_answer_shows_what_it_produced() {
        let engine = engine();
        // Fixed level ensures a reverse exercise is reachable; `ret` almost
        // never matches a nesting target, so it should render-but-mismatch.
        let mut probe = Rng::new(99);
        let exercise = generate(&engine, "rust", Level::Nesting, &ctx(), &mut probe)
            .expect("exercise generated");
        // Only assert the wrong-answer path when the generated exercise is
        // reverse (otherwise a numeric/free answer is graded differently).
        if exercise.direction != Direction::Reverse {
            return;
        }
        let mut session = Session::new(&engine, "rust", ctx(), Some(Level::Nesting), 99);
        let mut input = Cursor::new("ret\n:show\n:quit\n");
        let mut out: Vec<u8> = Vec::new();
        session.run(&mut input, &mut out).expect("session runs");
        let text = String::from_utf8(out).expect("utf8 output");
        assert!(text.contains("expands to") || text.contains("not quite"));
    }
}
