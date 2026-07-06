//! The pure trainer state machine.
//!
//! `Trainer` owns all session state and logic — exercise progression, grading,
//! hints, stats — but performs no I/O. Front ends (the ratatui TUI and the
//! line runner) feed it [`Event`]s and read back its state to render. Because
//! everything here is deterministic given a seed, the whole trainer can be
//! driven and asserted headlessly, which is what the tests below do.

use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::Engine;
use codemmet_core::snippet::render::TabstopMode;

use super::exercise::{Direction, Exercise, Level, generate};
use super::grade::{Outcome, grade_forward_choice, grade_forward_free, grade_reverse};
use super::hint::hints;
use super::rng::Rng;

/// Auto-reveal the answer after this many wrong attempts on one exercise.
const MAX_MISSES: u32 = 3;
/// Exercises presented per level in a guided lesson.
const DEFAULT_PER_LEVEL: u32 = 4;

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

/// The result of an exercise, shown on the feedback screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feedback {
    /// Whether the learner answered correctly.
    pub solved: bool,
    /// Whether the answer was revealed (`:show`, `:skip`, or too many misses).
    pub revealed: bool,
    /// A canonical answer for reference.
    pub answer: String,
}

/// What the trainer is currently presenting / waiting for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Showing a level's concept intro; [`Event::Advance`] begins its exercises.
    Intro(Level),
    /// Waiting for an answer to the current exercise.
    Answering,
    /// Showing the outcome of the current exercise; [`Event::Advance`] continues.
    Feedback(Feedback),
    /// The lesson plan is complete.
    Done,
}

/// An input event from a front end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Type a character into the answer buffer.
    Insert(char),
    /// Delete the last character of the answer buffer.
    Backspace,
    /// Submit the current answer buffer.
    Submit,
    /// Reveal the next progressive hint.
    Hint,
    /// Reveal the answer and move to feedback.
    Reveal,
    /// Skip this exercise (reveals the answer).
    Skip,
    /// Advance from an intro or feedback screen.
    Advance,
    /// End the session.
    Quit,
}

/// The trainer state machine.
pub struct Trainer<'a> {
    engine: &'a Engine,
    language: String,
    ctx: ExpansionContext,
    rng: Rng,

    /// The levels to play, in order.
    levels: Vec<Level>,
    /// Exercises per level.
    per_level: u32,
    /// Index into `levels`.
    level_idx: usize,
    /// Exercises completed at the current level.
    done_in_level: u32,

    stats: Stats,
    phase: Phase,

    /// The current exercise (present while [`Phase::Answering`]/`Feedback`).
    exercise: Option<Exercise>,
    /// The learner's in-progress answer.
    input: String,
    /// All staged hints for the current exercise.
    hints: Vec<String>,
    /// How many hints have been revealed.
    shown_hints: usize,
    /// Wrong attempts on the current exercise.
    misses: u32,
    /// A transient note (wrong/invalid/hint), shown while answering.
    message: Option<String>,
}

impl<'a> Trainer<'a> {
    /// A new guided-lesson trainer. `level` fixes a single level; `None` plays
    /// every level in order.
    pub fn new(
        engine: &'a Engine,
        language: impl Into<String>,
        ctx: ExpansionContext,
        level: Option<Level>,
        seed: u64,
    ) -> Self {
        let levels = level.map_or_else(|| Level::ALL.to_vec(), |l| vec![l]);
        let first = levels[0];
        Self {
            engine,
            language: language.into(),
            ctx,
            rng: Rng::new(seed),
            levels,
            per_level: DEFAULT_PER_LEVEL,
            level_idx: 0,
            done_in_level: 0,
            stats: Stats::default(),
            phase: Phase::Intro(first),
            exercise: None,
            input: String::new(),
            hints: Vec::new(),
            shown_hints: 0,
            misses: 0,
            message: None,
        }
    }

    // --- state, for rendering -------------------------------------------------

    /// The current phase.
    #[must_use]
    pub const fn phase(&self) -> &Phase {
        &self.phase
    }

    /// Whether the session is over.
    #[must_use]
    pub const fn is_done(&self) -> bool {
        matches!(self.phase, Phase::Done)
    }

    /// The current exercise, if one is active.
    #[must_use]
    pub const fn exercise(&self) -> Option<&Exercise> {
        self.exercise.as_ref()
    }

    /// The in-progress answer buffer.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The hints revealed so far.
    #[must_use]
    pub fn revealed_hints(&self) -> &[String] {
        &self.hints[..self.shown_hints]
    }

    /// The transient feedback note, if any.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The tally so far.
    #[must_use]
    pub const fn stats(&self) -> &Stats {
        &self.stats
    }

    /// The level being taught right now.
    #[must_use]
    pub fn current_level(&self) -> Level {
        match self.phase {
            Phase::Intro(level) => level,
            _ => self.levels[self.level_idx.min(self.levels.len() - 1)],
        }
    }

    /// Progress within the current level: `(completed, total)`.
    #[must_use]
    pub const fn level_progress(&self) -> (u32, u32) {
        (self.done_in_level, self.per_level)
    }

    /// Overall lesson progress: `(current level index, total levels)`.
    #[must_use]
    pub const fn plan_progress(&self) -> (usize, usize) {
        (self.level_idx, self.levels.len())
    }

    /// The live expansion of the current input, for a reverse exercise. `None`
    /// when the exercise is forward, the input is empty, or it does not render.
    #[must_use]
    pub fn live_preview(&self) -> Option<String> {
        let exercise = self.exercise.as_ref()?;
        if exercise.direction != Direction::Reverse || self.input.trim().is_empty() {
            return None;
        }
        let body = self
            .engine
            .expand(
                self.input.trim(),
                &self.language,
                &self.ctx,
                TabstopMode::Strip,
            )
            .ok()?
            .body;
        (!body.trim().is_empty()).then_some(body)
    }

    // --- events ---------------------------------------------------------------

    /// Feed an event to the state machine.
    pub fn handle(&mut self, event: Event) {
        match (&self.phase, event) {
            (_, Event::Quit) => self.phase = Phase::Done,
            (Phase::Intro(_) | Phase::Feedback(_), Event::Advance) => self.advance(),
            (Phase::Answering, event) => self.handle_answering(event),
            _ => {}
        }
    }

    /// Convenience for the line runner: set the buffer and submit in one step.
    pub fn submit_answer(&mut self, answer: &str) {
        if matches!(self.phase, Phase::Answering) {
            self.input = answer.trim().to_string();
            self.submit();
        }
    }

    fn handle_answering(&mut self, event: Event) {
        match event {
            Event::Insert(c) => {
                self.message = None;
                self.input.push(c);
            }
            Event::Backspace => {
                self.message = None;
                self.input.pop();
            }
            Event::Submit => self.submit(),
            Event::Hint => self.reveal_hint(),
            Event::Reveal | Event::Skip => self.reveal_answer(),
            _ => {}
        }
    }

    fn reveal_hint(&mut self) {
        if self.shown_hints < self.hints.len() {
            // The hint joins the persistent list (see `revealed_hints`); no
            // transient message needed.
            self.shown_hints += 1;
            self.message = None;
        } else {
            self.message = Some("no more hints — reveal the answer to move on".to_string());
        }
    }

    fn submit(&mut self) {
        let Some(exercise) = self.exercise.as_ref() else {
            return;
        };
        match self.grade(exercise) {
            Outcome::Correct => {
                self.stats.attempts += 1;
                self.stats.solved += 1;
                self.stats.streak += 1;
                self.stats.best_streak = self.stats.best_streak.max(self.stats.streak);
                self.enter_feedback(true, false);
            }
            Outcome::Wrong { .. } => {
                self.stats.attempts += 1;
                self.stats.streak = 0;
                self.misses += 1;
                if self.misses >= MAX_MISSES {
                    self.reveal_answer();
                } else {
                    self.message = Some("not quite — try again, or ask for a hint".to_string());
                }
            }
            Outcome::Invalid { message } => {
                self.message = Some(message);
            }
        }
    }

    /// Grade the current answer buffer against `exercise`.
    fn grade(&self, exercise: &Exercise) -> Outcome {
        let answer = self.input.trim();
        match exercise.direction {
            Direction::Reverse => grade_reverse(
                self.engine,
                &self.language,
                &self.ctx,
                answer,
                &exercise.target,
            ),
            Direction::Forward if exercise.is_multiple_choice() => match answer.parse::<usize>() {
                Ok(n) if (1..=exercise.choices.len()).contains(&n) => {
                    grade_forward_choice(n - 1, exercise.answer_index)
                }
                _ => Outcome::Invalid {
                    message: format!("enter a number between 1 and {}", exercise.choices.len()),
                },
            },
            Direction::Forward => grade_forward_free(answer, &exercise.target),
        }
    }

    fn reveal_answer(&mut self) {
        self.stats.streak = 0;
        self.enter_feedback(false, true);
    }

    /// A canonical answer string for the current exercise.
    fn canonical_answer(&self) -> String {
        let Some(exercise) = self.exercise.as_ref() else {
            return String::new();
        };
        match exercise.direction {
            Direction::Reverse => exercise.abbreviation.clone(),
            Direction::Forward if exercise.is_multiple_choice() => {
                format!("option {}", exercise.answer_index + 1)
            }
            Direction::Forward => exercise.target.clone(),
        }
    }

    fn enter_feedback(&mut self, solved: bool, revealed: bool) {
        self.message = None;
        self.phase = Phase::Feedback(Feedback {
            solved,
            revealed,
            answer: self.canonical_answer(),
        });
    }

    /// Move on from an intro or feedback screen: generate the next exercise,
    /// advance the level, or finish.
    fn advance(&mut self) {
        if matches!(self.phase, Phase::Feedback(_)) {
            self.done_in_level += 1;
            if self.done_in_level >= self.per_level {
                self.level_idx += 1;
                self.done_in_level = 0;
                if self.level_idx >= self.levels.len() {
                    self.phase = Phase::Done;
                    return;
                }
                self.phase = Phase::Intro(self.levels[self.level_idx]);
                return;
            }
        }
        self.start_exercise();
    }

    /// Generate and present an exercise at the current level.
    fn start_exercise(&mut self) {
        let level = self.levels[self.level_idx];
        let Some(exercise) = generate(self.engine, &self.language, level, &self.ctx, &mut self.rng)
        else {
            self.phase = Phase::Done;
            return;
        };
        self.hints = hints(&exercise);
        self.exercise = Some(exercise);
        self.input.clear();
        self.shown_hints = 0;
        self.misses = 0;
        self.message = None;
        self.stats.presented += 1;
        self.phase = Phase::Answering;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codemmet_core::context::IndentUnit;
    use codemmet_core::lang::registry::Registry;

    fn engine() -> Engine {
        Engine::with_registry(Registry::with_builtins().expect("builtins should load"))
    }

    fn ctx() -> ExpansionContext {
        ExpansionContext::plain(String::new(), IndentUnit::Spaces(4))
    }

    fn trainer(engine: &Engine, level: Option<Level>, seed: u64) -> Trainer<'_> {
        Trainer::new(engine, "rust", ctx(), level, seed)
    }

    /// The answer that solves an exercise, in the format the buffer expects.
    fn solution(exercise: &Exercise) -> String {
        match exercise.direction {
            Direction::Reverse => exercise.abbreviation.clone(),
            Direction::Forward if exercise.is_multiple_choice() => {
                (exercise.answer_index + 1).to_string()
            }
            Direction::Forward => exercise.target.clone(),
        }
    }

    fn type_str(trainer: &mut Trainer, text: &str) {
        for c in text.chars() {
            trainer.handle(Event::Insert(c));
        }
    }

    #[test]
    fn starts_on_the_intro_for_the_first_level() {
        let engine = engine();
        let t = trainer(&engine, Some(Level::Nesting), 1);
        assert_eq!(t.phase(), &Phase::Intro(Level::Nesting));
        assert!(t.exercise().is_none());
    }

    #[test]
    fn advance_from_intro_presents_an_exercise() {
        let engine = engine();
        let mut t = trainer(&engine, Some(Level::Nesting), 1);
        t.handle(Event::Advance);
        assert_eq!(t.phase(), &Phase::Answering);
        assert!(t.exercise().is_some());
        assert_eq!(t.stats().presented, 1);
    }

    #[test]
    fn typing_the_solution_solves_the_exercise() {
        let engine = engine();
        let seed = 7;
        // Replicate the first exercise the trainer will generate.
        let mut probe = Rng::new(seed);
        let exercise =
            generate(&engine, "rust", Level::Nesting, &ctx(), &mut probe).expect("generated");
        let answer = solution(&exercise);

        let mut t = trainer(&engine, Some(Level::Nesting), seed);
        t.handle(Event::Advance);
        assert_eq!(t.exercise(), Some(&exercise));
        type_str(&mut t, &answer);
        t.handle(Event::Submit);

        match t.phase() {
            Phase::Feedback(fb) => assert!(fb.solved && !fb.revealed),
            other => panic!("expected solved feedback, got {other:?}"),
        }
        assert_eq!(t.stats().solved, 1);
        assert_eq!(t.stats().best_streak, 1);
    }

    #[test]
    fn backspace_edits_the_buffer() {
        let engine = engine();
        let mut t = trainer(&engine, Some(Level::SingleTrigger), 1);
        t.handle(Event::Advance);
        type_str(&mut t, "abc");
        t.handle(Event::Backspace);
        assert_eq!(t.input(), "ab");
    }

    #[test]
    fn hints_reveal_progressively_then_run_out() {
        let engine = engine();
        let mut t = trainer(&engine, Some(Level::Nesting), 3);
        t.handle(Event::Advance);
        let total = t.exercise().expect("exercise").concepts.len();
        assert!(total > 0);
        // Nesting exercises always have at least the trigger + operator hints.
        let expected = crate::train::hint::hints(t.exercise().expect("exercise")).len();
        assert!(expected >= 1);
        t.handle(Event::Hint);
        assert_eq!(t.revealed_hints().len(), 1);
        // Exhaust hints; the count caps and the message says so.
        for _ in 0..8 {
            t.handle(Event::Hint);
        }
        assert_eq!(t.revealed_hints().len(), expected);
        assert!(t.message().is_some_and(|m| m.contains("no more hints")));
    }

    #[test]
    fn reveal_moves_to_revealed_feedback_without_solving() {
        let engine = engine();
        let mut t = trainer(&engine, Some(Level::Nesting), 5);
        t.handle(Event::Advance);
        t.handle(Event::Reveal);
        match t.phase() {
            Phase::Feedback(fb) => {
                assert!(!fb.solved && fb.revealed);
                assert!(!fb.answer.is_empty());
            }
            other => panic!("expected revealed feedback, got {other:?}"),
        }
        assert_eq!(t.stats().solved, 0);
    }

    #[test]
    fn three_wrong_answers_auto_reveal() {
        let engine = engine();
        let mut t = trainer(&engine, Some(Level::Nesting), 11);
        t.handle(Event::Advance);
        // A junk-but-parseable reverse answer, or a wrong MC pick.
        for _ in 0..MAX_MISSES {
            // `ret` renders for reverse; a wrong number for MC.
            let guess = if t.exercise().is_some_and(Exercise::is_multiple_choice) {
                // Pick an index that is not the answer.
                let ex = t.exercise().expect("exercise");
                let wrong = if ex.answer_index == 0 { 2 } else { 1 };
                wrong.to_string()
            } else {
                "ret".to_string()
            };
            t.submit_answer(&guess);
        }
        assert!(
            matches!(t.phase(), Phase::Feedback(fb) if fb.revealed),
            "expected auto-reveal after {MAX_MISSES} misses, phase: {:?}",
            t.phase(),
        );
    }

    #[test]
    fn live_preview_tracks_reverse_input() {
        let engine = engine();
        // Find a reverse exercise.
        let mut t = trainer(&engine, Some(Level::Nesting), 2);
        t.handle(Event::Advance);
        while t
            .exercise()
            .is_some_and(|e| e.direction != Direction::Reverse)
        {
            t.handle(Event::Reveal);
            t.handle(Event::Advance);
        }
        assert!(t.live_preview().is_none(), "empty input has no preview");
        type_str(&mut t, "pr");
        let preview = t.live_preview().expect("pr renders");
        assert!(preview.contains("println!"));
    }

    #[test]
    fn a_single_level_plan_finishes_after_its_exercises() {
        let engine = engine();
        let mut t = trainer(&engine, Some(Level::SingleTrigger), 1);
        // Intro -> begin.
        t.handle(Event::Advance);
        // Reveal-and-advance through the whole level.
        let mut guard = 0;
        while !t.is_done() {
            guard += 1;
            assert!(guard < 100, "plan did not terminate");
            match t.phase() {
                Phase::Answering => t.handle(Event::Reveal),
                Phase::Intro(_) | Phase::Feedback(_) => t.handle(Event::Advance),
                Phase::Done => {}
            }
        }
        let (_, total) = t.level_progress();
        assert_eq!(t.stats().presented, total);
    }

    #[test]
    fn quit_ends_the_session_from_any_phase() {
        let engine = engine();
        let mut t = trainer(&engine, Some(Level::Nesting), 1);
        t.handle(Event::Advance);
        t.handle(Event::Quit);
        assert!(t.is_done());
    }
}
