//! Progressive hint derivation.
//!
//! Hints come straight from an exercise's concepts, staged from least to most
//! revealing: first the triggers involved, then the operators. A final reveal
//! of the canonical answer is handled by the runner, not here.

use super::exercise::{Concept, Exercise};

/// The staged hints for an exercise, least revealing first.
#[must_use]
pub fn hints(exercise: &Exercise) -> Vec<String> {
    let mut triggers: Vec<&str> = Vec::new();
    let mut operators: Vec<char> = Vec::new();
    for concept in &exercise.concepts {
        match concept {
            Concept::Trigger(t) => {
                if !triggers.contains(&t.as_str()) {
                    triggers.push(t);
                }
            }
            Concept::Operator(op) => {
                if !operators.contains(op) {
                    operators.push(*op);
                }
            }
        }
    }

    let mut out = Vec::new();
    if !triggers.is_empty() {
        out.push(format!("Triggers involved: {}", triggers.join(", ")));
    }
    if !operators.is_empty() {
        let named: Vec<String> = operators
            .iter()
            .map(|op| format!("`{op}` ({})", operator_name(*op)))
            .collect();
        out.push(format!("Operators involved: {}", named.join(", ")));
    }
    out
}

/// The human name of a composition operator.
const fn operator_name(op: char) -> &'static str {
    match op {
        '>' => "child",
        '+' => "sibling",
        '*' => "repeat",
        '^' => "climb-up",
        '(' | ')' => "group",
        _ => "operator",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exercise::{Direction, Level};

    fn exercise(concepts: Vec<Concept>) -> Exercise {
        Exercise {
            language: "rust".into(),
            level: Level::Nesting,
            direction: Direction::Reverse,
            abbreviation: "fn:main>pr".into(),
            target: String::new(),
            choices: Vec::new(),
            answer_index: 0,
            concepts,
        }
    }

    #[test]
    fn triggers_come_before_operators() {
        let ex = exercise(vec![
            Concept::Trigger("fn".into()),
            Concept::Trigger("pr".into()),
            Concept::Operator('>'),
        ]);
        let hints = hints(&ex);
        assert_eq!(hints.len(), 2);
        assert!(hints[0].contains("fn") && hints[0].contains("pr"));
        assert!(hints[1].contains('>') && hints[1].contains("child"));
    }

    #[test]
    fn triggers_are_deduplicated_preserving_order() {
        let ex = exercise(vec![
            Concept::Trigger("pr".into()),
            Concept::Trigger("pr".into()),
        ]);
        let hints = hints(&ex);
        assert_eq!(hints, vec!["Triggers involved: pr".to_string()]);
    }

    #[test]
    fn no_operators_means_a_single_hint() {
        let ex = exercise(vec![Concept::Trigger("ret".into())]);
        assert_eq!(hints(&ex).len(), 1);
    }
}
