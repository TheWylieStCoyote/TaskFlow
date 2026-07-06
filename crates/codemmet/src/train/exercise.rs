//! Exercise model and auto-generation.
//!
//! Everything here is pure: given an [`Engine`], a language, a [`Level`], and a
//! seeded [`Rng`], `generate` builds a renderable exercise by composing real
//! triggers with the composition operators and rendering the result through the
//! engine. Because the target is produced by the engine, a generated exercise
//! can never disagree with actual expansion behavior.

use codemmet_core::config::loader::{CompiledSnippet, SnippetSet};
use codemmet_core::config::schema::ParamKind;
use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::Engine;
use codemmet_core::snippet::ir::Span;
use codemmet_core::snippet::render::TabstopMode;

use super::rng::Rng;

/// How many times `generate` retries composing a renderable abbreviation
/// before falling back to a bare trigger.
const GEN_ATTEMPTS: usize = 48;
/// How many times decoy generation retries before giving up.
const DECOY_ATTEMPTS: usize = 48;
/// Target number of decoys for a multiple-choice forward exercise.
const DECOY_TARGET: usize = 3;

/// Identifier argument pool for generated calls.
const IDENTS: &[&str] = &["i", "x", "item", "node", "value"];
/// Integer argument pool.
const INTS: &[&str] = &["3", "5", "10"];
/// String argument pool.
const STRS: &[&str] = &["msg", "label", "name"];

/// A staged difficulty level: composition operators first, then vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// A single trigger with no arguments (`pr`, `ret`).
    SingleTrigger,
    /// A trigger with arguments (`fn:main`, `fori:i,10`).
    Arguments,
    /// Child nesting with `>` (`fn:main>pr`).
    Nesting,
    /// Siblings with `+` (`if+el`).
    Siblings,
    /// Repetition with `*` (`arm*3`).
    Repeat,
    /// Climb-up with `^` (`fn:main>pr^ret`).
    ClimbUp,
    /// Grouping with `()` (`(fn:main>pr)+ret`).
    Grouping,
}

impl Level {
    /// Every level, in teaching order.
    pub const ALL: [Self; 7] = [
        Self::SingleTrigger,
        Self::Arguments,
        Self::Nesting,
        Self::Siblings,
        Self::Repeat,
        Self::ClimbUp,
        Self::Grouping,
    ];

    /// The 1-based level number.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::SingleTrigger => 1,
            Self::Arguments => 2,
            Self::Nesting => 3,
            Self::Siblings => 4,
            Self::Repeat => 5,
            Self::ClimbUp => 6,
            Self::Grouping => 7,
        }
    }

    /// The level for a 1-based number, if in range.
    #[must_use]
    pub const fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::SingleTrigger),
            2 => Some(Self::Arguments),
            3 => Some(Self::Nesting),
            4 => Some(Self::Siblings),
            5 => Some(Self::Repeat),
            6 => Some(Self::ClimbUp),
            7 => Some(Self::Grouping),
            _ => None,
        }
    }

    /// A short human title.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::SingleTrigger => "single triggers",
            Self::Arguments => "arguments",
            Self::Nesting => "nesting (>)",
            Self::Siblings => "siblings (+)",
            Self::Repeat => "repeat (*)",
            Self::ClimbUp => "climb-up (^)",
            Self::Grouping => "grouping ( )",
        }
    }

    /// A teaching blurb shown on the lesson's intro screen.
    #[must_use]
    pub const fn blurb(self) -> &'static str {
        match self {
            Self::SingleTrigger => {
                "A trigger is a short keyword that expands to a construct.\n\
                 Type the trigger on its own, e.g. `pr` or `ret`."
            }
            Self::Arguments => {
                "Triggers take arguments. Glue them on (`fori10`, `fn:main`) or pass\n\
                 them explicitly after a colon, comma-separated (`fori:i,10`)."
            }
            Self::Nesting => {
                "`>` nests the right side inside the left as its child:\n\
                 `fn:main>pr` puts a println inside a main function."
            }
            Self::Siblings => {
                "`+` places two constructs side by side, as siblings:\n\
                 `if+el` yields an if followed by an else."
            }
            Self::Repeat => {
                "`*n` repeats a construct n times, each with its own tabstops:\n\
                 `arm*3` produces three match arms."
            }
            Self::ClimbUp => {
                "`^` climbs back up out of the current child, so the next construct\n\
                 is a sibling of the parent: `fn:main>pr^ret`."
            }
            Self::Grouping => {
                "`( )` groups a whole subtree so operators apply to it as a unit:\n\
                 `(fn:main>pr)+ret` — the group, then a sibling."
            }
        }
    }
}

/// The exercise direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Given target code, produce an abbreviation.
    Reverse,
    /// Given an abbreviation, predict the expansion.
    Forward,
}

/// A concept an exercise touches: used for hints and as a spaced-repetition
/// item (hence the ordering and serde derives).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum Concept {
    /// A trigger word.
    Trigger(String),
    /// A composition operator character.
    Operator(char),
}

impl Concept {
    /// A stable key for use as a map key in persisted review state.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Trigger(t) => format!("t:{t}"),
            Self::Operator(op) => format!("o:{op}"),
        }
    }

    /// The level whose exercises best drill this concept.
    #[must_use]
    pub const fn level(&self) -> Level {
        match self {
            Self::Trigger(_) => Level::Arguments,
            Self::Operator('>') => Level::Nesting,
            Self::Operator('+') => Level::Siblings,
            Self::Operator('*') => Level::Repeat,
            Self::Operator('^') => Level::ClimbUp,
            Self::Operator(_) => Level::Grouping,
        }
    }
}

/// A single generated exercise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    /// The language id this exercise is for.
    pub language: String,
    /// The difficulty level.
    pub level: Level,
    /// Reverse (produce the abbreviation) or forward (predict the code).
    pub direction: Direction,
    /// A canonical abbreviation that produces `target`.
    pub abbreviation: String,
    /// The strip-mode expansion of `abbreviation`.
    pub target: String,
    /// Multiple-choice options for a forward exercise; empty means free-type.
    pub choices: Vec<String>,
    /// Index of the correct option within `choices` (meaningless when empty).
    pub answer_index: usize,
    /// Triggers and operators involved, for hints.
    pub concepts: Vec<Concept>,
}

impl Exercise {
    /// Whether the forward exercise is answered by choosing from `choices`.
    #[must_use]
    pub fn is_multiple_choice(&self) -> bool {
        self.direction == Direction::Forward && !self.choices.is_empty()
    }
}

/// Triggers of a language bucketed by shape, borrowed from a [`SnippetSet`].
struct Vocab<'a> {
    /// Triggers with no parameters.
    no_param: Vec<&'a CompiledSnippet>,
    /// Triggers with at least one parameter.
    with_param: Vec<&'a CompiledSnippet>,
    /// Triggers whose template has a `${child}` slot (block constructs).
    blocks: Vec<&'a CompiledSnippet>,
    /// Triggers with no `${child}` slot (statement-like leaves).
    leaves: Vec<&'a CompiledSnippet>,
}

impl<'a> Vocab<'a> {
    fn from_set(set: &'a SnippetSet) -> Self {
        let mut v = Self {
            no_param: Vec::new(),
            with_param: Vec::new(),
            blocks: Vec::new(),
            leaves: Vec::new(),
        };
        for s in set.iter() {
            if s.params.is_empty() {
                v.no_param.push(s);
            } else {
                v.with_param.push(s);
            }
            if has_child_slot(s) {
                v.blocks.push(s);
            } else {
                v.leaves.push(s);
            }
        }
        v
    }
}

/// Whether a snippet's template contains a `${child}` slot.
fn has_child_slot(snip: &CompiledSnippet) -> bool {
    snip.template
        .lines
        .iter()
        .any(|line| line.spans.iter().any(|s| matches!(s, Span::ChildSlot)))
}

/// A pool value for a parameter of the given kind.
fn arg_for(kind: ParamKind, rng: &mut Rng) -> String {
    let pool = match kind {
        ParamKind::Ident => IDENTS,
        ParamKind::Int => INTS,
        ParamKind::Str => STRS,
    };
    rng.choice(pool).copied().unwrap_or("x").to_string()
}

/// Render one trigger call, filling any parameters with explicit args, and
/// record the trigger as a concept.
fn call(snip: &CompiledSnippet, rng: &mut Rng, concepts: &mut Vec<Concept>) -> String {
    concepts.push(Concept::Trigger(snip.trigger.clone()));
    if snip.params.is_empty() {
        snip.trigger.clone()
    } else {
        let args: Vec<String> = snip.params.iter().map(|p| arg_for(p.kind, rng)).collect();
        format!("{}:{}", snip.trigger, args.join(","))
    }
}

/// Compose a candidate abbreviation for `level`, or `None` when the vocabulary
/// lacks a trigger of the shape a level needs.
fn compose(vocab: &Vocab, level: Level, rng: &mut Rng) -> Option<(String, Vec<Concept>)> {
    let mut concepts = Vec::new();
    let abbr = match level {
        Level::SingleTrigger => {
            let s = rng.choice(&vocab.no_param).copied()?;
            call(s, rng, &mut concepts)
        }
        Level::Arguments => {
            let s = rng.choice(&vocab.with_param).copied()?;
            call(s, rng, &mut concepts)
        }
        Level::Nesting => {
            let a = rng.choice(&vocab.blocks).copied()?;
            let b = rng.choice(&vocab.leaves).copied()?;
            let sa = call(a, rng, &mut concepts);
            let sb = call(b, rng, &mut concepts);
            concepts.push(Concept::Operator('>'));
            format!("{sa}>{sb}")
        }
        Level::Siblings => {
            let a = rng.choice(&vocab.leaves).copied()?;
            let b = rng.choice(&vocab.leaves).copied()?;
            let sa = call(a, rng, &mut concepts);
            let sb = call(b, rng, &mut concepts);
            concepts.push(Concept::Operator('+'));
            format!("{sa}+{sb}")
        }
        Level::Repeat => {
            let a = rng.choice(&vocab.leaves).copied()?;
            let sa = call(a, rng, &mut concepts);
            let n = rng.choice(&[2_usize, 3]).copied().unwrap_or(2);
            concepts.push(Concept::Operator('*'));
            format!("{sa}*{n}")
        }
        Level::ClimbUp => {
            let a = rng.choice(&vocab.blocks).copied()?;
            let b = rng.choice(&vocab.leaves).copied()?;
            let c = rng.choice(&vocab.leaves).copied()?;
            let sa = call(a, rng, &mut concepts);
            let sb = call(b, rng, &mut concepts);
            let sc = call(c, rng, &mut concepts);
            concepts.push(Concept::Operator('>'));
            concepts.push(Concept::Operator('^'));
            format!("{sa}>{sb}^{sc}")
        }
        Level::Grouping => {
            let a = rng.choice(&vocab.blocks).copied()?;
            let b = rng.choice(&vocab.leaves).copied()?;
            let c = rng.choice(&vocab.leaves).copied()?;
            let sa = call(a, rng, &mut concepts);
            let sb = call(b, rng, &mut concepts);
            let sc = call(c, rng, &mut concepts);
            concepts.push(Concept::Operator('('));
            concepts.push(Concept::Operator('+'));
            format!("({sa}>{sb})+{sc}")
        }
    };
    Some((abbr, concepts))
}

/// The rendering context threaded through generation: an engine, a language,
/// and an expansion context.
struct Gen<'a> {
    engine: &'a Engine,
    language: &'a str,
    ctx: &'a ExpansionContext,
}

impl Gen<'_> {
    /// Render `abbr` to strip-mode text, or `None` when it fails or is empty.
    fn render(&self, abbr: &str) -> Option<String> {
        let body = self
            .engine
            .expand(abbr, self.language, self.ctx, TabstopMode::Strip)
            .ok()?
            .body;
        if body.trim().is_empty() {
            None
        } else {
            Some(body)
        }
    }

    /// Distinct multi-line decoy expansions at `level` that differ from `target`.
    fn decoys(&self, vocab: &Vocab, level: Level, target: &str, rng: &mut Rng) -> Vec<String> {
        let mut decoys: Vec<String> = Vec::new();
        for _ in 0..DECOY_ATTEMPTS {
            if decoys.len() >= DECOY_TARGET {
                break;
            }
            let Some((abbr, _)) = compose(vocab, level, rng) else {
                continue;
            };
            let Some(body) = self.render(&abbr) else {
                continue;
            };
            if body.contains('\n') && body != target && !decoys.contains(&body) {
                decoys.push(body);
            }
        }
        decoys
    }

    /// Assemble a finished exercise, choosing the direction and (for multi-line
    /// forward exercises) building the multiple-choice options.
    fn finish(
        &self,
        vocab: &Vocab,
        level: Level,
        abbr: String,
        target: String,
        concepts: Vec<Concept>,
        rng: &mut Rng,
    ) -> Exercise {
        // Forward multi-line exercises are multiple choice; a single-line target
        // is free-typed. A multi-line target with no distinct decoys falls back
        // to a reverse exercise rather than asking for many typed lines.
        let (direction, choices, answer_index) = if rng.coin() {
            if target.contains('\n') {
                let decoys = self.decoys(vocab, level, &target, rng);
                if decoys.is_empty() {
                    (Direction::Reverse, Vec::new(), 0)
                } else {
                    let (choices, answer_index) = build_choices(&target, decoys, rng);
                    (Direction::Forward, choices, answer_index)
                }
            } else {
                (Direction::Forward, Vec::new(), 0)
            }
        } else {
            (Direction::Reverse, Vec::new(), 0)
        };
        Exercise {
            language: self.language.to_string(),
            level,
            direction,
            abbreviation: abbr,
            target,
            choices,
            answer_index,
            concepts,
        }
    }
}

/// Shuffle `target` in among `decoys`, returning the options and the target's
/// index.
fn build_choices(target: &str, mut all: Vec<String>, rng: &mut Rng) -> (Vec<String>, usize) {
    all.push(target.to_string());
    // Fisher-Yates.
    for i in (1..all.len()).rev() {
        all.swap(i, rng.below(i + 1));
    }
    let idx = all.iter().position(|c| c == target).unwrap_or(0);
    (all, idx)
}

/// Generate a renderable exercise for `language` at `level`.
///
/// Returns `None` only when the language is unknown or has no renderable
/// triggers at all.
#[must_use]
pub fn generate(
    engine: &Engine,
    language: &str,
    level: Level,
    ctx: &ExpansionContext,
    rng: &mut Rng,
) -> Option<Exercise> {
    let set = &engine.registry().language(language)?.set;
    let vocab = Vocab::from_set(set);
    let generator = Gen {
        engine,
        language,
        ctx,
    };

    for _ in 0..GEN_ATTEMPTS {
        if let Some((abbr, concepts)) = compose(&vocab, level, rng)
            && let Some(target) = generator.render(&abbr)
        {
            return Some(generator.finish(&vocab, level, abbr, target, concepts, rng));
        }
    }

    // Fallback: the simplest renderable single trigger, so a session never
    // stalls even for a sparse pack.
    for s in &vocab.no_param {
        if let Some(target) = generator.render(&s.trigger) {
            return Some(generator.finish(
                &vocab,
                Level::SingleTrigger,
                s.trigger.clone(),
                target,
                vec![Concept::Trigger(s.trigger.clone())],
                rng,
            ));
        }
    }
    None
}

/// Generate an exercise that drills a specific concept (for spaced repetition).
///
/// A trigger item yields an exercise built around that exact trigger; an
/// operator item yields a normal exercise at the operator's level. Returns
/// `None` when the concept can no longer be rendered (e.g. a trigger the pack
/// dropped).
#[must_use]
pub fn generate_for_concept(
    engine: &Engine,
    language: &str,
    concept: &Concept,
    ctx: &ExpansionContext,
    rng: &mut Rng,
) -> Option<Exercise> {
    let Concept::Trigger(name) = concept else {
        return generate(engine, language, concept.level(), ctx, rng);
    };
    let set = &engine.registry().language(language)?.set;
    let snippet = set.get(name)?;
    let vocab = Vocab::from_set(set);
    let generator = Gen {
        engine,
        language,
        ctx,
    };
    let mut concepts = Vec::new();
    let abbr = call(snippet, rng, &mut concepts);
    let target = generator.render(&abbr)?;
    let level = if snippet.params.is_empty() {
        Level::SingleTrigger
    } else {
        Level::Arguments
    };
    Some(generator.finish(&vocab, level, abbr, target, concepts, rng))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::train::grade::{Outcome, grade_reverse};
    use codemmet_core::context::IndentUnit;
    use codemmet_core::lang::registry::Registry;

    fn engine() -> Engine {
        Engine::with_registry(Registry::with_builtins().expect("builtins should load"))
    }

    fn ctx() -> ExpansionContext {
        ExpansionContext::plain(String::new(), IndentUnit::Spaces(4))
    }

    #[test]
    fn generates_a_renderable_self_consistent_exercise_at_every_level() {
        let engine = engine();
        let ctx = ctx();
        for level in Level::ALL {
            // A few seeds so we exercise different vocabulary picks.
            for seed in 0..8_u64 {
                let mut rng = Rng::new(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1));
                let ex = generate(&engine, "rust", level, &ctx, &mut rng)
                    .expect("rust has renderable triggers");
                assert!(!ex.target.trim().is_empty(), "empty target at {level:?}");
                // The canonical abbreviation must grade correct against its own
                // target — generation and grading agree by construction.
                let outcome = grade_reverse(&engine, "rust", &ctx, &ex.abbreviation, &ex.target);
                assert_eq!(
                    outcome,
                    Outcome::Correct,
                    "canonical answer {:?} did not reproduce target at {level:?}",
                    ex.abbreviation,
                );
            }
        }
    }

    #[test]
    fn generation_is_deterministic_for_a_seed() {
        let engine = engine();
        let ctx = ctx();
        let mut a = Rng::new(12345);
        let mut b = Rng::new(12345);
        let ea =
            generate(&engine, "rust", Level::Nesting, &ctx, &mut a).expect("exercise generated");
        let eb =
            generate(&engine, "rust", Level::Nesting, &ctx, &mut b).expect("exercise generated");
        assert_eq!(ea, eb);
    }

    #[test]
    fn multiple_choice_answer_index_points_at_the_target() {
        // Search seeds for a multi-line forward exercise and check the answer.
        let engine = engine();
        let ctx = ctx();
        let mut found = false;
        for seed in 0..200_u64 {
            let mut rng = Rng::new(seed.wrapping_add(1));
            let ex = generate(&engine, "rust", Level::Nesting, &ctx, &mut rng)
                .expect("exercise generated");
            if ex.is_multiple_choice() {
                assert_eq!(ex.choices[ex.answer_index], ex.target);
                assert!(ex.choices.len() >= 2);
                found = true;
                break;
            }
        }
        assert!(found, "no multiple-choice exercise generated in 200 seeds");
    }

    #[test]
    fn levels_round_trip_through_numbers() {
        for level in Level::ALL {
            assert_eq!(Level::from_number(level.number()), Some(level));
        }
        assert_eq!(Level::from_number(0), None);
        assert_eq!(Level::from_number(8), None);
    }
}
