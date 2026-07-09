//! Hand-authored lesson progressions.
//!
//! Guided lessons draw from a curated, deliberately-ordered set of
//! abbreviations per level so the experience is stable and pedagogical, unlike
//! the randomized drills. Each supported language has its own progression,
//! keyed by canonical language id (the trainer normalizes aliases before
//! looking up). Every abbreviation here is rendered through the real engine
//! (and a test asserts each one renders), so curated content can never drift
//! from actual behavior.

use super::exercise::Level;

/// The curated abbreviations for `language` at `level`, in teaching order.
/// Empty when the language has no curated progression (falls back to generated).
#[must_use]
pub fn curated(language: &str, level: Level) -> &'static [&'static str] {
    match language {
        "rust" => rust(level),
        "python" => python(level),
        "c" => c(level),
        "cpp" => cpp(level),
        "javascript" => javascript(level),
        "typescript" => typescript(level),
        "go" => go(level),
        _ => &[],
    }
}

/// Every language id that ships a curated progression.
#[cfg(test)]
const CURATED_LANGUAGES: &[&str] = &[
    "rust",
    "python",
    "c",
    "cpp",
    "javascript",
    "typescript",
    "go",
];

const fn rust(level: Level) -> &'static [&'static str] {
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
        Level::Composite => &["dstruct:Point", "denum:Color", "newtype:Meters", "testmod"],
        Level::DataDriven => &[
            "struct:P>field*[x:i32,y:f64]",
            "impl:Point>fn*[new,len,area]",
            "enum:Color>variant*[Red,Green,Blue]",
            "mod*[parser,lexer]>mtest*[empty,unicode]",
        ],
    }
}

const fn python(level: Level) -> &'static [&'static str] {
    match level {
        Level::SingleTrigger => &["pr", "ret", "todo", "main"],
        Level::Arguments => &["def:greet", "class:Widget", "fori:i,10"],
        Level::Nesting => &["def:main>pr", "fori:i,10>pr", "if>ret", "while>pr"],
        Level::Siblings => &["pr+ret", "def:a+def:b"],
        Level::Repeat => &["pr*3", "def:main>pr*3"],
        Level::ClimbUp => &["def:main>fori:i,10>pr^ret", "def:run>if>pr^ret"],
        Level::Grouping => &["(def:main>pr)+def:helper", "(fori:i,10>pr)+ret"],
        Level::Composite => &["classp:Point", "ctxmgr:Timer"],
        Level::DataDriven => &[
            "class:Shape>def*[area,perim]",
            "class:P>field*[x,y]",
            "match:cmd>arm*[Quit,Move]",
        ],
    }
}

const fn c(level: Level) -> &'static [&'static str] {
    match level {
        Level::SingleTrigger => &["pr", "ret", "todo", "main"],
        Level::Arguments => &["fn:compute", "struct:Point", "fori:i,10"],
        Level::Nesting => &["fn:main>pr", "fori:i,10>pr", "if>ret", "while>pr"],
        Level::Siblings => &["pr+ret", "fn:a+fn:b"],
        Level::Repeat => &["pr*3", "fn:main>pr*3"],
        Level::ClimbUp => &["fn:main>fori:i,10>pr^ret", "fn:run>if>pr^ret"],
        Level::Grouping => &["(fn:main>pr)+fn:helper", "(fori:i,10>pr)+ret"],
        Level::Composite => &["hstruct:Node", "henum:Color"],
        Level::DataDriven => &[
            "struct:P>field*[x:int,y:int]",
            "enum:Color>variant*[Red,Green]",
            "switch:cmd>case*[A,B]",
        ],
    }
}

const fn cpp(level: Level) -> &'static [&'static str] {
    match level {
        Level::SingleTrigger => &["pr", "ret", "todo", "main"],
        Level::Arguments => &["fn:compute", "class:Widget", "fori:i,10", "ns:app"],
        Level::Nesting => &[
            "fn:main>pr",
            "fori:i,10>pr",
            "if>ret",
            "class:Widget>fn:run",
        ],
        Level::Siblings => &["pr+ret", "fn:a+fn:b"],
        Level::Repeat => &["pr*3", "fn:main>pr*3"],
        Level::ClimbUp => &["fn:main>fori:i,10>pr^ret", "fn:run>if>pr^ret"],
        Level::Grouping => &["(fn:main>pr)+fn:helper", "(fori:i,10>pr)+ret"],
        Level::Composite => &["hclass:Widget", "nsclass:Shape", "henum:Mode"],
        Level::DataDriven => &[
            "struct:P>field*[x:int,y:int]",
            "enum:Mode>variant*[On,Off]",
            "switch:cmd>case*[A,B]",
        ],
    }
}

const fn javascript(level: Level) -> &'static [&'static str] {
    match level {
        Level::SingleTrigger => &["pr", "ret", "todo", "arrow"],
        Level::Arguments => &["fn:greet", "class:Widget", "fori:i,10"],
        Level::Nesting => &[
            "fn:main>pr",
            "fori:i,10>pr",
            "if>ret",
            "class:Widget>fn:run",
        ],
        Level::Siblings => &["pr+ret", "fn:a+fn:b"],
        Level::Repeat => &["pr*3", "fn:main>pr*3"],
        Level::ClimbUp => &["fn:main>fori:i,10>pr^ret", "fn:run>if>pr^ret"],
        Level::Grouping => &["(fn:main>pr)+fn:helper", "(fori:i,10>pr)+ret"],
        Level::Composite => &["classc:Widget"],
        Level::DataDriven => &["switch:cmd>case*[A,B]", "class:S>fn*[area,perim]"],
    }
}

const fn typescript(level: Level) -> &'static [&'static str] {
    match level {
        Level::SingleTrigger => &["pr", "ret", "todo", "arrow"],
        Level::Arguments => &["fn:greet", "class:Widget", "fori:i,10", "int:Shape"],
        Level::Nesting => &[
            "fn:main>pr",
            "fori:i,10>pr",
            "if>ret",
            "class:Widget>fn:run",
        ],
        Level::Siblings => &["pr+ret", "fn:a+fn:b"],
        Level::Repeat => &["pr*3", "fn:main>pr*3"],
        Level::ClimbUp => &["fn:main>fori:i,10>pr^ret", "fn:run>if>pr^ret"],
        Level::Grouping => &["(fn:main>pr)+fn:helper", "(fori:i,10>pr)+ret"],
        Level::Composite => &["classc:Shape"],
        Level::DataDriven => &[
            "int:Shape>field*[x:number,y:number]",
            "enum:Color>variant*[Red,Green]",
            "switch:cmd>case*[A,B]",
        ],
    }
}

const fn go(level: Level) -> &'static [&'static str] {
    match level {
        Level::SingleTrigger => &["pr", "ret", "todo", "main"],
        Level::Arguments => &["fn:run", "struct:Point", "fori:i,10", "iface:Shape"],
        Level::Nesting => &["fn:main>pr", "fori:i,10>pr", "if>ret", "for>pr"],
        Level::Siblings => &["pr+ret", "fn:a+fn:b"],
        Level::Repeat => &["pr*3", "fn:main>pr*3"],
        Level::ClimbUp => &["fn:main>fori:i,10>pr^ret", "fn:run>if>pr^ret"],
        Level::Grouping => &["(fn:main>pr)+fn:helper", "(fori:i,10>pr)+ret"],
        Level::Composite => &["stringer:Point", "errtype:ParseError", "maincmd"],
        Level::DataDriven => &["struct:P>field*[X:int,Y:int]", "switch:cmd>case*[A,B]"],
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
    fn every_curated_abbreviation_renders() {
        let engine = Engine::with_registry(Registry::with_builtins().expect("builtins"));
        let ctx = ExpansionContext::plain(String::new(), IndentUnit::Spaces(4));
        for language in CURATED_LANGUAGES {
            for level in Level::ALL {
                let set = curated(language, level);
                assert!(
                    !set.is_empty(),
                    "{language} has no curated set for {level:?}",
                );
                for abbr in set {
                    let body = engine
                        .expand(abbr, language, &ctx, TabstopMode::Strip)
                        .unwrap_or_else(|err| {
                            panic!("curated `{abbr}` ({language}, {level:?}) failed: {err}")
                        })
                        .body;
                    assert!(
                        !body.trim().is_empty(),
                        "curated `{abbr}` ({language}, {level:?}) rendered empty",
                    );
                }
            }
        }
    }

    #[test]
    fn unknown_language_has_no_curated_set() {
        assert!(curated("cobol", Level::SingleTrigger).is_empty());
        assert!(curated("fortran", Level::Nesting).is_empty());
    }
}
