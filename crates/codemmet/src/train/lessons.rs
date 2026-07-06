//! Hand-authored lesson progressions.
//!
//! Guided lessons draw from a curated, deliberately-ordered set of
//! abbreviations per level so the experience is stable and pedagogical, unlike
//! the randomized drills. Only languages with a curated set get one; others
//! fall back to generated exercises. Every abbreviation here is rendered
//! through the real engine (and a test asserts each one renders), so curated
//! content can never drift from actual behavior.

use super::exercise::Level;

/// The curated abbreviations for `language` at `level`, in teaching order.
/// Empty when the language has no curated progression (falls back to generated).
#[must_use]
pub fn curated(language: &str, level: Level) -> &'static [&'static str] {
    if language != "rust" {
        return &[];
    }
    match level {
        Level::SingleTrigger => &["pr", "ret", "vec", "main", "derive"],
        Level::Arguments => &[
            "fn:greet",
            "let:x",
            "letm:total",
            "fori:i,10",
            "struct:Point",
        ],
        Level::Nesting => &["fn:main>pr", "fori:i,10>pr", "if>ret", "fn:run>let:x"],
        Level::Siblings => &["pr+ret", "let:x+let:y", "fn:a+fn:b"],
        Level::Repeat => &["pr*3", "let:x*2", "fn:main>pr*3"],
        Level::ClimbUp => &["fn:main>fori:i,10>pr^ret", "fn:run>if>pr^ret"],
        Level::Grouping => &["(fn:main>pr)+fn:helper", "(fori:i,10>pr)+ret"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codemmet_core::context::{ExpansionContext, IndentUnit};
    use codemmet_core::engine::Engine;
    use codemmet_core::lang::registry::Registry;
    use codemmet_core::snippet::render::TabstopMode;

    #[test]
    fn every_curated_rust_abbreviation_renders() {
        let engine = Engine::with_registry(Registry::with_builtins().expect("builtins"));
        let ctx = ExpansionContext::plain(String::new(), IndentUnit::Spaces(4));
        for level in Level::ALL {
            for abbr in curated("rust", level) {
                let body = engine
                    .expand(abbr, "rust", &ctx, TabstopMode::Strip)
                    .unwrap_or_else(|err| panic!("curated `{abbr}` ({level:?}) failed: {err}"))
                    .body;
                assert!(
                    !body.trim().is_empty(),
                    "curated `{abbr}` ({level:?}) rendered empty",
                );
            }
        }
    }

    #[test]
    fn unknown_language_has_no_curated_set() {
        assert!(curated("cobol", Level::SingleTrigger).is_empty());
        assert!(curated("python", Level::Nesting).is_empty());
    }
}
