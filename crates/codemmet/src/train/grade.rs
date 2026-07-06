//! Output-equivalence grading.
//!
//! A reverse answer is correct iff it expands (strip mode) to the same visible
//! code as the target — any abbreviation that produces the target passes, not
//! just the canonical one. Comparison ignores per-line trailing whitespace,
//! which empty tabstop spots would otherwise leave behind.

use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::Engine;
use codemmet_core::snippet::render::TabstopMode;

/// The result of grading an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The answer produces the target.
    Correct,
    /// The answer is valid but produces different code.
    Wrong {
        /// What the learner's answer actually rendered to (reverse exercises).
        rendered: Option<String>,
    },
    /// The answer could not be parsed or rendered at all (reverse exercises).
    Invalid {
        /// The engine's error message.
        message: String,
    },
}

/// Compare two expansions ignoring per-line trailing whitespace.
#[must_use]
pub fn equivalent(a: &str, b: &str) -> bool {
    normalize(a) == normalize(b)
}

/// Trim trailing whitespace from each line so equivalence is about the visible
/// code, not the invisible spaces empty tabstops leave.
fn normalize(s: &str) -> String {
    s.lines().map(str::trim_end).collect::<Vec<_>>().join("\n")
}

/// Grade a reverse answer: render the learner's abbreviation and compare it to
/// the target.
#[must_use]
pub fn grade_reverse(
    engine: &Engine,
    language: &str,
    ctx: &ExpansionContext,
    answer: &str,
    target: &str,
) -> Outcome {
    match engine.expand(answer.trim(), language, ctx, TabstopMode::Strip) {
        Ok(exp) if equivalent(&exp.body, target) => Outcome::Correct,
        Ok(exp) => Outcome::Wrong {
            rendered: Some(exp.body),
        },
        Err(err) => Outcome::Invalid {
            message: err.to_string(),
        },
    }
}

/// Grade a free-typed forward answer: compare the typed code to the target.
#[must_use]
pub fn grade_forward_free(answer: &str, target: &str) -> Outcome {
    if equivalent(answer, target) {
        Outcome::Correct
    } else {
        Outcome::Wrong { rendered: None }
    }
}

/// Grade a multiple-choice forward answer by option index.
#[must_use]
pub const fn grade_forward_choice(chosen: usize, answer_index: usize) -> Outcome {
    if chosen == answer_index {
        Outcome::Correct
    } else {
        Outcome::Wrong { rendered: None }
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

    fn target(engine: &Engine, abbr: &str) -> String {
        engine
            .expand(abbr, "rust", &ctx(), TabstopMode::Strip)
            .expect("expands")
            .body
    }

    #[test]
    fn normalize_ignores_trailing_whitespace() {
        assert!(equivalent("a  \nb\t\nc", "a\nb\nc"));
        assert!(!equivalent("a\nb", "a\n  b"));
    }

    #[test]
    fn canonical_answer_is_correct() {
        let engine = engine();
        let t = target(&engine, "fori10>pr");
        assert_eq!(
            grade_reverse(&engine, "rust", &ctx(), "fori10>pr", &t),
            Outcome::Correct,
        );
    }

    #[test]
    fn any_equivalent_abbreviation_is_correct() {
        // Glued and explicit argument forms expand identically, so both pass.
        let engine = engine();
        let t = target(&engine, "fori10");
        assert_eq!(
            grade_reverse(&engine, "rust", &ctx(), "fori:i,10", &t),
            Outcome::Correct,
        );
    }

    #[test]
    fn different_code_is_wrong_not_invalid() {
        let engine = engine();
        let t = target(&engine, "fori10");
        match grade_reverse(&engine, "rust", &ctx(), "ret", &t) {
            Outcome::Wrong { rendered: Some(_) } => {}
            other => panic!("expected Wrong with rendered output, got {other:?}"),
        }
    }

    #[test]
    fn unparseable_answer_is_invalid() {
        let engine = engine();
        let t = target(&engine, "pr");
        match grade_reverse(&engine, "rust", &ctx(), "definitely_not_a_trigger", &t) {
            Outcome::Invalid { .. } => {}
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn forward_grading() {
        assert_eq!(
            grade_forward_free("let x = 1;", "let x = 1;  "),
            Outcome::Correct,
        );
        assert_eq!(
            grade_forward_free("nope", "let x = 1;"),
            Outcome::Wrong { rendered: None },
        );
        assert_eq!(grade_forward_choice(2, 2), Outcome::Correct);
        assert_eq!(
            grade_forward_choice(1, 2),
            Outcome::Wrong { rendered: None },
        );
    }
}
