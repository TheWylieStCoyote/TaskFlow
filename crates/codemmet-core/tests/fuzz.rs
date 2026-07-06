//! Property-based fuzzing of the parsers and the expansion pipeline.
//!
//! The abbreviation parser is the untrusted-input surface (users and the LSP
//! feed it arbitrary text), so the overriding invariant is: **it must never
//! panic**, whatever the input. These tests throw both arbitrary text and
//! structured, abbreviation-shaped input at `parse`, the template/body parser,
//! and the full `expand` pipeline. For coverage-guided fuzzing, see `fuzz/`.

use codemmet_core::abbr;
use codemmet_core::context::{ExpansionContext, IndentUnit};
use codemmet_core::engine::Engine;
use codemmet_core::lang::registry::Registry;
use codemmet_core::snippet::render::TabstopMode;
use codemmet_core::snippet::template::parse_template;
use proptest::prelude::*;
use std::sync::OnceLock;

fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::with_registry(Registry::with_builtins().expect("builtins")))
}

/// One token drawn from the real abbreviation alphabet, so generated strings
/// reach deep parser paths instead of being rejected at the first character.
fn abbr_token() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::sample::select(vec![
            "pr", "ret", "fn", "fori", "if", "el", "match", "arm", "struct", "impl", "let", "main",
            "mod", "tmod", "test", "vec",
        ])
        .prop_map(String::from),
        prop::sample::select(vec![">", "+", "*", "^", "(", ")", ":", ","]).prop_map(String::from),
        "[a-zA-Z_][a-zA-Z0-9_]*", // identifiers
        "[0-9]{1,4}",             // small numbers
        prop::sample::select(vec![" ", "\t"]).prop_map(String::from), // whitespace
    ]
}

/// A string built from abbreviation tokens (numbers kept small so a legitimate
/// `*n` repeat doesn't ask the engine to build an enormous expansion).
fn abbr_string() -> impl Strategy<Value = String> {
    prop::collection::vec(abbr_token(), 0..24).prop_map(|tokens| tokens.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// The parser never panics on arbitrary text.
    #[test]
    fn parse_never_panics_on_arbitrary_text(s in ".*") {
        let _ = abbr::parse(&s);
    }

    /// ...nor on structured, abbreviation-shaped input (deeper coverage).
    #[test]
    fn parse_never_panics_on_structured_input(s in abbr_string()) {
        let _ = abbr::parse(&s);
    }

    /// The whole expand pipeline (parse + resolve + compose + render) never
    /// panics, in either tabstop mode.
    #[test]
    fn expand_never_panics(s in abbr_string()) {
        let ctx = ExpansionContext::plain(String::new(), IndentUnit::Spaces(4));
        let _ = engine().expand(&s, "rust", &ctx, TabstopMode::Strip);
        let _ = engine().expand(&s, "rust", &ctx, TabstopMode::Keep);
    }

    /// The template/body parser never panics on arbitrary text.
    #[test]
    fn template_parser_never_panics(s in ".*") {
        let _ = parse_template(&s);
    }
}
