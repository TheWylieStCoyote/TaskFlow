//! The pure trainer state machine.
//!
//! `Trainer` owns all session state and logic — exercise progression across the
//! four modes, grading, hints, spaced-repetition scheduling, stats — but
//! performs no I/O. Front ends (the ratatui TUI and the line runner) feed it
//! [`Event`]s and read back its state to render. Everything is deterministic
//! given a seed and an injected `now`, so the whole trainer is driven and
//! asserted headlessly in the tests below.

use std::collections::{BTreeSet, VecDeque};

use codemmet_core::context::{ExpansionContext, IndentUnit};
use codemmet_core::engine::Engine;
use codemmet_core::snippet::render::TabstopMode;

use super::exercise::{
    Concept, Direction, Exercise, Level, from_abbreviation, generate, generate_for_concept,
};
use super::grade::{Outcome, grade_forward_choice, grade_forward_free, grade_reverse};
use super::hint::hints;
use super::lessons;
use super::review::ReviewState;
use super::rng::Rng;

/// Auto-reveal the answer after this many wrong attempts on one exercise.
const MAX_MISSES: u32 = 3;
/// Exercises presented per level in a guided lesson.
const DEFAULT_PER_LEVEL: u32 = 4;

/// Which trainer experience to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// A guided progression: a concept intro per level, then its exercises.
    #[default]
    Lessons,
    /// Endless randomized exercises at a level (or mixed).
    Drills,
    /// Only the spaced-repetition items that are due.
    Review,
    /// Free play: type abbreviations and see them expand, no scoring.
    Sandbox,
}

impl Mode {
    /// A short human name.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Lessons => "lessons",
            Self::Drills => "drills",
            Self::Review => "review",
            Self::Sandbox => "sandbox",
        }
    }
}

/// How to construct a [`Trainer`].
#[derive(Debug, Clone)]
pub struct Config {
    /// Language id to train.
    pub language: String,
    /// Which experience to run.
    pub mode: Mode,
    /// A single level to focus (lessons) or drill; `None` means all/mixed.
    pub level: Option<Level>,
    /// Seed for deterministic generation.
    pub seed: u64,
    /// The current time, in Unix seconds, for spaced-repetition scheduling.
    pub now: u64,
}

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
    /// Free-play sandbox: input expands live, nothing is graded.
    Sandbox,
    /// The session is complete.
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
#[derive(Debug)]
pub struct Trainer<'a> {
    engine: &'a Engine,
    language: String,
    ctx: ExpansionContext,
    rng: Rng,
    mode: Mode,
    now: u64,

    /// Levels this run may draw from (one when focused, else all).
    levels: Vec<Level>,
    /// Index into `levels` (lessons progression).
    level_idx: usize,
    /// Exercises completed at the current level (lessons).
    done_in_level: u32,
    /// Levels finished this session (lessons).
    completed: BTreeSet<u8>,

    /// The spaced-repetition schedule (seeded from disk, updated as we grade).
    review: ReviewState,
    /// The queue of due concepts to review, in review mode.
    review_queue: VecDeque<Concept>,

    /// Expansions produced in the sandbox, newest last.
    sandbox_log: Vec<(String, String)>,

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
    /// A transient note (wrong/invalid), shown while answering.
    message: Option<String>,
}

impl<'a> Trainer<'a> {
    /// A new trainer for `config`, seeded with the persisted `review` schedule.
    pub fn new(engine: &'a Engine, config: Config, review: ReviewState) -> Self {
        let Config {
            language,
            mode,
            level,
            seed,
            now,
        } = config;
        // Normalize to the canonical language id (e.g. `js` -> `javascript`) so
        // curated lessons are found regardless of the alias the user typed.
        let language = engine
            .registry()
            .language(&language)
            .map_or(language, |lang| lang.profile.id.to_string());
        let levels = level.map_or_else(|| Level::ALL.to_vec(), |l| vec![l]);
        let mut trainer = Self {
            engine,
            language,
            ctx: ExpansionContext::plain(String::new(), IndentUnit::Spaces(4)),
            rng: Rng::new(seed),
            mode,
            now,
            levels,
            level_idx: 0,
            done_in_level: 0,
            completed: BTreeSet::new(),
            review,
            review_queue: VecDeque::new(),
            sandbox_log: Vec::new(),
            stats: Stats::default(),
            phase: Phase::Done,
            exercise: None,
            input: String::new(),
            hints: Vec::new(),
            shown_hints: 0,
            misses: 0,
            message: None,
        };
        trainer.ctx = ExpansionContext::plain(String::new(), trainer.default_indent());
        trainer.begin();
        trainer
    }

    /// The active language's default indent unit.
    fn default_indent(&self) -> IndentUnit {
        self.engine
            .registry()
            .language(&self.language)
            .map_or(IndentUnit::Spaces(4), |lang| {
                lang.profile.default_indent.clone()
            })
    }

    /// Set the initial phase for the mode.
    fn begin(&mut self) {
        match self.mode {
            Mode::Lessons => self.phase = Phase::Intro(self.levels[0]),
            Mode::Drills => self.start_exercise(),
            Mode::Review => {
                self.review_queue = self.review.due(self.now).into();
                self.start_exercise();
            }
            Mode::Sandbox => self.phase = Phase::Sandbox,
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

    /// The mode this trainer is running.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
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

    /// The spaced-repetition schedule (updated as exercises are graded).
    #[must_use]
    pub const fn review_state(&self) -> &ReviewState {
        &self.review
    }

    /// How many spaced-repetition items are due as of this session's clock.
    #[must_use]
    pub fn due_count(&self) -> usize {
        self.review.due_count(self.now)
    }

    /// Lesson levels completed this session.
    #[must_use]
    pub const fn completed_levels(&self) -> &BTreeSet<u8> {
        &self.completed
    }

    /// The sandbox scrollback of `(abbreviation, expansion)`, newest last.
    #[must_use]
    pub fn sandbox_log(&self) -> &[(String, String)] {
        &self.sandbox_log
    }

    /// The level being taught / drilled right now.
    #[must_use]
    pub fn current_level(&self) -> Level {
        match &self.phase {
            Phase::Intro(level) => *level,
            _ => self.exercise.as_ref().map_or_else(
                || self.levels[self.level_idx.min(self.levels.len() - 1)],
                |exercise| exercise.level,
            ),
        }
    }

    /// A short sub-header describing progress, tailored to the mode.
    #[must_use]
    pub fn header_note(&self) -> String {
        match self.mode {
            Mode::Lessons => {
                format!(
                    "exercise {}/{}",
                    self.done_in_level + 1,
                    self.level_target()
                )
            }
            Mode::Drills => format!("drill #{}", self.stats.presented.max(1)),
            Mode::Review => format!("review — {} left", self.review_queue.len() + 1),
            Mode::Sandbox => "sandbox".to_string(),
        }
    }

    /// How many exercises the current lesson level presents: the size of its
    /// curated set, or [`DEFAULT_PER_LEVEL`] when there is none.
    fn level_target(&self) -> u32 {
        let level = self.levels[self.level_idx.min(self.levels.len() - 1)];
        let curated = lessons::curated(&self.language, level).len();
        if curated == 0 {
            DEFAULT_PER_LEVEL
        } else {
            u32::try_from(curated).unwrap_or(DEFAULT_PER_LEVEL)
        }
    }

    /// The live expansion of the current input — for a reverse exercise or in
    /// the sandbox. `None` when there is nothing to preview.
    #[must_use]
    pub fn live_preview(&self) -> Option<String> {
        if self.input.trim().is_empty() {
            return None;
        }
        let show = matches!(self.phase, Phase::Sandbox)
            || self
                .exercise
                .as_ref()
                .is_some_and(|e| e.direction == Direction::Reverse);
        if show {
            self.expand(self.input.trim())
        } else {
            None
        }
    }

    // --- events ---------------------------------------------------------------

    /// Feed an event to the state machine.
    pub fn handle(&mut self, event: Event) {
        match (&self.phase, event) {
            (_, Event::Quit) => self.phase = Phase::Done,
            (Phase::Intro(_) | Phase::Feedback(_), Event::Advance) => self.advance(),
            (Phase::Answering, event) => self.handle_answering(event),
            (Phase::Sandbox, event) => self.handle_sandbox(event),
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

    /// Convenience for the line runner: expand one sandbox line.
    pub fn sandbox_input(&mut self, line: &str) {
        if matches!(self.phase, Phase::Sandbox) {
            self.input = line.trim().to_string();
            self.handle_sandbox(Event::Submit);
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

    fn handle_sandbox(&mut self, event: Event) {
        match event {
            Event::Insert(c) => {
                self.message = None;
                self.input.push(c);
            }
            Event::Backspace => {
                self.message = None;
                self.input.pop();
            }
            Event::Submit => {
                let abbr = self.input.trim().to_string();
                if !abbr.is_empty() {
                    match self.expand(&abbr) {
                        Some(expansion) => self.sandbox_log.push((abbr, expansion)),
                        None => {
                            self.message = Some("that abbreviation does not expand".to_string());
                        }
                    }
                }
                self.input.clear();
            }
            _ => {}
        }
    }

    fn reveal_hint(&mut self) {
        if self.shown_hints < self.hints.len() {
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
        // Record spaced-repetition outcomes: a clean solve promotes the
        // concepts; a reveal or a solve-after-misses demotes them.
        let correct = solved && self.misses == 0;
        if let Some(concepts) = self.exercise.as_ref().map(|e| e.concepts.clone()) {
            for concept in &concepts {
                self.review.record(concept, correct, self.now);
            }
        }
        self.message = None;
        self.phase = Phase::Feedback(Feedback {
            solved,
            revealed,
            answer: self.canonical_answer(),
        });
    }

    /// Move on from an intro or feedback screen.
    fn advance(&mut self) {
        // In lessons, count the finished exercise and roll levels over.
        if self.mode == Mode::Lessons && matches!(self.phase, Phase::Feedback(_)) {
            self.done_in_level += 1;
            if self.done_in_level >= self.level_target() {
                self.completed.insert(self.levels[self.level_idx].number());
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

    /// Generate and present the next exercise for the mode.
    fn start_exercise(&mut self) {
        let exercise = match self.mode {
            Mode::Lessons => self.next_lesson_exercise(),
            Mode::Drills => {
                let level = self.pick_level();
                generate(self.engine, &self.language, level, &self.ctx, &mut self.rng)
            }
            Mode::Review => self.next_review_exercise(),
            Mode::Sandbox => None,
        };
        let Some(exercise) = exercise else {
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

    /// The next lesson exercise: the curated abbreviation for this position,
    /// falling back to a generated one when curated content runs out.
    fn next_lesson_exercise(&mut self) -> Option<Exercise> {
        let level = self.levels[self.level_idx];
        let curated = lessons::curated(&self.language, level);
        let abbr = curated.get(self.done_in_level as usize).copied();
        let from_curated = abbr.and_then(|abbr| {
            from_abbreviation(
                self.engine,
                &self.language,
                level,
                abbr,
                None,
                &self.ctx,
                &mut self.rng,
            )
        });
        from_curated
            .or_else(|| generate(self.engine, &self.language, level, &self.ctx, &mut self.rng))
    }

    /// The level for the next drill exercise.
    fn pick_level(&mut self) -> Level {
        match self.mode {
            Mode::Drills if self.levels.len() > 1 => self
                .rng
                .choice(&self.levels)
                .copied()
                .unwrap_or(Level::SingleTrigger),
            _ => self.levels[self.level_idx.min(self.levels.len() - 1)],
        }
    }

    /// Pull the next renderable review exercise off the due queue.
    fn next_review_exercise(&mut self) -> Option<Exercise> {
        while let Some(concept) = self.review_queue.pop_front() {
            if let Some(exercise) = generate_for_concept(
                self.engine,
                &self.language,
                &concept,
                &self.ctx,
                &mut self.rng,
            ) {
                return Some(exercise);
            }
        }
        None
    }

    /// Render `abbr` to strip-mode text, or `None` if it fails / is empty.
    fn expand(&self, abbr: &str) -> Option<String> {
        let body = self
            .engine
            .expand(abbr, &self.language, &self.ctx, TabstopMode::Strip)
            .ok()?
            .body;
        (!body.trim().is_empty()).then_some(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codemmet_core::lang::registry::Registry;

    fn engine() -> Engine {
        Engine::with_registry(Registry::with_builtins().expect("builtins should load"))
    }

    fn config(mode: Mode, level: Option<Level>, seed: u64) -> Config {
        Config {
            language: "rust".to_string(),
            mode,
            level,
            seed,
            now: 1_000_000,
        }
    }

    fn lessons(engine: &Engine, level: Option<Level>, seed: u64) -> Trainer<'_> {
        Trainer::new(
            engine,
            config(Mode::Lessons, level, seed),
            ReviewState::default(),
        )
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

    fn type_str(trainer: &mut Trainer, text: &str) {
        for c in text.chars() {
            trainer.handle(Event::Insert(c));
        }
    }

    #[test]
    fn lessons_start_on_the_intro() {
        let engine = engine();
        let t = lessons(&engine, Some(Level::Nesting), 1);
        assert_eq!(t.phase(), &Phase::Intro(Level::Nesting));
        assert!(t.exercise().is_none());
    }

    #[test]
    fn advance_from_intro_presents_an_exercise() {
        let engine = engine();
        let mut t = lessons(&engine, Some(Level::Nesting), 1);
        t.handle(Event::Advance);
        assert_eq!(t.phase(), &Phase::Answering);
        assert!(t.exercise().is_some());
        assert_eq!(t.stats().presented, 1);
    }

    #[test]
    fn typing_the_solution_solves_the_exercise() {
        let engine = engine();
        let seed = 7;
        // Lessons draw from the curated set, so replicate the first curated
        // Nesting exercise (built the same way, from the same fresh seed).
        let first = lessons::curated("rust", Level::Nesting)[0];
        let mut probe = Rng::new(seed);
        let exercise = from_abbreviation(
            &engine,
            "rust",
            Level::Nesting,
            first,
            None,
            &probe_ctx(),
            &mut probe,
        )
        .expect("curated renders");
        let answer = solution(&exercise);

        let mut t = lessons(&engine, Some(Level::Nesting), seed);
        t.handle(Event::Advance);
        assert_eq!(t.exercise(), Some(&exercise));
        type_str(&mut t, &answer);
        t.handle(Event::Submit);

        match t.phase() {
            Phase::Feedback(fb) => assert!(fb.solved && !fb.revealed),
            other => panic!("expected solved feedback, got {other:?}"),
        }
        assert_eq!(t.stats().solved, 1);
    }

    fn probe_ctx() -> ExpansionContext {
        ExpansionContext::plain(String::new(), IndentUnit::Spaces(4))
    }

    #[test]
    fn hints_reveal_progressively_then_run_out() {
        let engine = engine();
        let mut t = lessons(&engine, Some(Level::Nesting), 3);
        t.handle(Event::Advance);
        let expected = crate::hint::hints(t.exercise().expect("exercise")).len();
        assert!(expected >= 1);
        t.handle(Event::Hint);
        assert_eq!(t.revealed_hints().len(), 1);
        for _ in 0..8 {
            t.handle(Event::Hint);
        }
        assert_eq!(t.revealed_hints().len(), expected);
        assert!(t.message().is_some_and(|m| m.contains("no more hints")));
    }

    #[test]
    fn reveal_moves_to_revealed_feedback_without_solving() {
        let engine = engine();
        let mut t = lessons(&engine, Some(Level::Nesting), 5);
        t.handle(Event::Advance);
        t.handle(Event::Reveal);
        match t.phase() {
            Phase::Feedback(fb) => assert!(!fb.solved && fb.revealed && !fb.answer.is_empty()),
            other => panic!("expected revealed feedback, got {other:?}"),
        }
        assert_eq!(t.stats().solved, 0);
    }

    #[test]
    fn a_single_level_lesson_finishes_after_its_exercises() {
        let engine = engine();
        let mut t = lessons(&engine, Some(Level::SingleTrigger), 1);
        t.handle(Event::Advance);
        let mut guard = 0;
        while !t.is_done() {
            guard += 1;
            assert!(guard < 100, "plan did not terminate");
            match t.phase() {
                Phase::Answering => t.handle(Event::Reveal),
                _ => t.handle(Event::Advance),
            }
        }
        // The single-trigger level presents its whole curated set (5 items).
        assert_eq!(t.stats().presented, t.level_target());
        assert_eq!(t.level_target(), 5);
        assert!(
            t.completed_levels()
                .contains(&Level::SingleTrigger.number())
        );
    }

    #[test]
    fn drills_start_answering_and_never_finish_on_their_own() {
        let engine = engine();
        let mut t = Trainer::new(
            &engine,
            config(Mode::Drills, Some(Level::Nesting), 4),
            ReviewState::default(),
        );
        assert_eq!(t.phase(), &Phase::Answering);
        // Reveal + advance a bunch; drills keep producing exercises.
        for _ in 0..12 {
            t.handle(Event::Reveal);
            t.handle(Event::Advance);
            assert_eq!(t.phase(), &Phase::Answering);
        }
        assert!(t.stats().presented >= 12);
    }

    #[test]
    fn review_replays_due_items_then_finishes() {
        let engine = engine();
        let now = 5_000_000;
        // Seed two due items.
        let mut review = ReviewState::default();
        review.record(&Concept::Trigger("pr".into()), false, now); // due now
        review.record(&Concept::Operator('>'), false, now); // due now
        let config = Config {
            language: "rust".into(),
            mode: Mode::Review,
            level: None,
            seed: 2,
            now,
        };
        let mut t = Trainer::new(&engine, config, review);
        assert_eq!(t.phase(), &Phase::Answering, "a due item is presented");

        let mut guard = 0;
        while !t.is_done() {
            guard += 1;
            assert!(guard < 50);
            match t.phase() {
                Phase::Answering => t.handle(Event::Reveal),
                _ => t.handle(Event::Advance),
            }
        }
        assert!(t.stats().presented >= 1);
    }

    #[test]
    fn review_with_nothing_due_is_immediately_done() {
        let engine = engine();
        let config = Config {
            language: "rust".into(),
            mode: Mode::Review,
            level: None,
            seed: 1,
            now: 0,
        };
        let t = Trainer::new(&engine, config, ReviewState::default());
        assert!(t.is_done());
        assert_eq!(t.stats().presented, 0);
    }

    #[test]
    fn solving_updates_the_review_schedule() {
        let engine = engine();
        let mut t = lessons(&engine, Some(Level::Nesting), 5);
        t.handle(Event::Advance);
        assert_eq!(t.review_state().due_count(u64::MAX), 0);
        t.handle(Event::Reveal); // records the exercise's concepts (as misses)
        // Revealed => demoted => due now.
        assert!(t.review_state().due_count(1_000_000) >= 1);
    }

    #[test]
    fn sandbox_expands_input_and_logs_it() {
        let engine = engine();
        let mut t = Trainer::new(
            &engine,
            config(Mode::Sandbox, None, 1),
            ReviewState::default(),
        );
        assert_eq!(t.phase(), &Phase::Sandbox);
        type_str(&mut t, "pr");
        assert!(t.live_preview().is_some_and(|p| p.contains("println!")));
        t.handle(Event::Submit);
        assert_eq!(t.sandbox_log().len(), 1);
        assert!(t.sandbox_log()[0].1.contains("println!"));
        assert_eq!(t.input(), "", "buffer clears after submit");
    }

    #[test]
    fn quit_ends_the_session_from_any_phase() {
        let engine = engine();
        let mut t = lessons(&engine, Some(Level::Nesting), 1);
        t.handle(Event::Advance);
        t.handle(Event::Quit);
        assert!(t.is_done());
    }
}
