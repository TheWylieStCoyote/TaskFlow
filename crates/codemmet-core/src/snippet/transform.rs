//! Case transforms: the `|name` suffix in `${…}` references.
//!
//! A transform re-cases *known* text — a data-driven item, a member, or a
//! bound argument — into one naming convention. All transforms are total:
//! any input produces some reasonable output, so callers never need a
//! fallback path. See `CASE_TRANSFORMS_PLAN.md` for the design.
//!
//! Tokenization (shared by everything except [`CaseTransform::Lower`]):
//!
//! 1. A run of leading underscores is preserved verbatim as a prefix
//!    (`_maxSize|snake` → `_max_size`).
//! 2. `_`, `-`, and whitespace split tokens (the separator is dropped;
//!    empty tokens from doubled separators vanish).
//! 3. Within a run, a new token starts before an uppercase letter that
//!    follows a non-uppercase character (`maxSize` → `max`,`Size`;
//!    `utf8String` → `utf8`,`String`), and before the last capital of an
//!    acronym run followed by a lowercase letter (`HTTPServer` →
//!    `HTTP`,`Server`).
//! 4. Digits never start a token split on their own (`vec3` is one token).
//!
//! Case mapping is ASCII-only; non-ASCII characters pass through unchanged
//! and are caseless for boundary detection (they never split a token).

/// A named case transform, as written after `|` in `${value|snake}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseTransform {
    /// `snake`: tokens lowercased, joined with `_` (`maxSize` → `max_size`).
    Snake,
    /// `camel`: like [`Self::Pascal`] with the first token all-lowercase
    /// (`max_size` → `maxSize`).
    Camel,
    /// `pascal`: each token capitalized, joined (`max_size` → `MaxSize`).
    Pascal,
    /// `upper`: tokens uppercased, joined with `_` — `SCREAMING_SNAKE`
    /// (`maxSize` → `MAX_SIZE`).
    Upper,
    /// `kebab`: tokens lowercased, joined with `-` (`maxSize` → `max-size`).
    Kebab,
    /// `lower`: a plain ASCII lowercase fold of the whole string, no
    /// re-joining (`Red` → `red`, `MAX_SIZE` → `max_size`).
    Lower,
}

/// Every transform name, for error messages, sorted.
pub const TRANSFORM_NAMES: &[&str] = &["camel", "kebab", "lower", "pascal", "snake", "upper"];

impl CaseTransform {
    /// Look a transform up by its `|name` spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "snake" => Some(Self::Snake),
            "camel" => Some(Self::Camel),
            "pascal" => Some(Self::Pascal),
            "upper" => Some(Self::Upper),
            "kebab" => Some(Self::Kebab),
            "lower" => Some(Self::Lower),
            _ => None,
        }
    }

    /// The transform's `|name` spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Snake => "snake",
            Self::Camel => "camel",
            Self::Pascal => "pascal",
            Self::Upper => "upper",
            Self::Kebab => "kebab",
            Self::Lower => "lower",
        }
    }

    /// Re-case `text` into this transform's convention.
    #[must_use]
    pub fn apply(self, text: &str) -> String {
        if self == Self::Lower {
            return text.to_ascii_lowercase();
        }
        let (prefix, rest) = split_leading_underscores(text);
        let tokens = split_words(rest);
        let mut out = String::with_capacity(text.len() + prefix.len());
        out.push_str(prefix);
        for (i, token) in tokens.iter().enumerate() {
            match self {
                Self::Snake | Self::Kebab | Self::Upper => {
                    if i > 0 {
                        out.push(if self == Self::Kebab { '-' } else { '_' });
                    }
                    for c in token.chars() {
                        out.push(if self == Self::Upper {
                            c.to_ascii_uppercase()
                        } else {
                            c.to_ascii_lowercase()
                        });
                    }
                }
                Self::Pascal | Self::Camel => {
                    let lower_first = self == Self::Camel && i == 0;
                    for (j, c) in token.chars().enumerate() {
                        out.push(if j == 0 && !lower_first {
                            c.to_ascii_uppercase()
                        } else {
                            c.to_ascii_lowercase()
                        });
                    }
                }
                // Handled by the early return above.
                Self::Lower => unreachable!(),
            }
        }
        out
    }
}

/// Split off the run of leading underscores, preserved verbatim.
fn split_leading_underscores(text: &str) -> (&str, &str) {
    let rest = text.trim_start_matches('_');
    (&text[..text.len() - rest.len()], rest)
}

/// Rough character class for word-boundary detection. Non-ASCII is
/// *caseless* ([`Class::Other`]): it never creates a boundary on either
/// side, so `héllo` stays one token and transforms stay idempotent on text
/// the ASCII case fold cannot change.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Upper,
    Lower,
    Digit,
    Other,
}

const fn classify(c: char) -> Class {
    if c.is_ascii_uppercase() {
        Class::Upper
    } else if c.is_ascii_lowercase() {
        Class::Lower
    } else if c.is_ascii_digit() {
        Class::Digit
    } else {
        Class::Other
    }
}

/// Split `text` into word tokens per the module rules. Separators are
/// dropped; empty tokens never appear.
fn split_words(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    for segment in text.split(['_', '-'].as_slice()) {
        for piece in segment.split_whitespace() {
            split_case_runs(piece, &mut tokens);
        }
    }
    tokens
}

/// Split one separator-free piece at its case boundaries.
fn split_case_runs<'a>(piece: &'a str, tokens: &mut Vec<&'a str>) {
    let chars: Vec<(usize, char)> = piece.char_indices().collect();
    let mut start = 0;
    for i in 1..chars.len() {
        let (offset, cur) = chars[i];
        if classify(cur) != Class::Upper {
            continue;
        }
        let prev = classify(chars[i - 1].1);
        // lower/digit → Upper starts a new token (`maxSize`, `utf8String`).
        let after_lower = matches!(prev, Class::Lower | Class::Digit);
        // The last capital of an acronym run followed by a lowercase letter
        // starts a new token (`HTTPServer` → `HTTP`,`Server`).
        let acronym_end = prev == Class::Upper
            && chars
                .get(i + 1)
                .is_some_and(|&(_, next)| classify(next) == Class::Lower);
        if after_lower || acronym_end {
            tokens.push(&piece[start..offset]);
            start = offset;
        }
    }
    if start < piece.len() {
        tokens.push(&piece[start..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_name_round_trip() {
        for &name in TRANSFORM_NAMES {
            let transform = CaseTransform::parse(name).expect("listed name parses");
            assert_eq!(transform.name(), name);
        }
        assert_eq!(CaseTransform::parse("bogus"), None);
        assert_eq!(CaseTransform::parse("Snake"), None);
    }

    /// One row of expected outputs: (input, snake, camel, pascal, upper, kebab, lower).
    const CASES: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
        (
            "maxSize", "max_size", "maxSize", "MaxSize", "MAX_SIZE", "max-size", "maxsize",
        ),
        (
            "max_size", "max_size", "maxSize", "MaxSize", "MAX_SIZE", "max-size", "max_size",
        ),
        (
            "HTTPServer",
            "http_server",
            "httpServer",
            "HttpServer",
            "HTTP_SERVER",
            "http-server",
            "httpserver",
        ),
        (
            "XMLHttpRequest",
            "xml_http_request",
            "xmlHttpRequest",
            "XmlHttpRequest",
            "XML_HTTP_REQUEST",
            "xml-http-request",
            "xmlhttprequest",
        ),
        (
            "utf8String",
            "utf8_string",
            "utf8String",
            "Utf8String",
            "UTF8_STRING",
            "utf8-string",
            "utf8string",
        ),
        (
            "Vec3D", "vec3_d", "vec3D", "Vec3D", "VEC3_D", "vec3-d", "vec3d",
        ),
        (
            "_leading", "_leading", "_leading", "_Leading", "_LEADING", "_leading", "_leading",
        ),
        // Non-ASCII passes through unchanged (ASCII-only case mapping) and
        // never splits a token.
        (
            "__двa__x",
            "__двa_x",
            "__двaX",
            "__двaX",
            "__двA_X",
            "__двa-x",
            "__двa__x",
        ),
        (
            "kebab-in", "kebab_in", "kebabIn", "KebabIn", "KEBAB_IN", "kebab-in", "kebab-in",
        ),
        (
            "SCREAMING_IN",
            "screaming_in",
            "screamingIn",
            "ScreamingIn",
            "SCREAMING_IN",
            "screaming-in",
            "screaming_in",
        ),
        ("a b", "a_b", "aB", "AB", "A_B", "a-b", "a b"),
        ("x", "x", "x", "X", "X", "x", "x"),
        ("", "", "", "", "", "", ""),
        ("FOO", "foo", "foo", "Foo", "FOO", "foo", "foo"),
    ];

    #[test]
    fn transform_table() {
        for &(input, snake, camel, pascal, upper, kebab, lower) in CASES {
            assert_eq!(CaseTransform::Snake.apply(input), snake, "snake({input})");
            assert_eq!(CaseTransform::Camel.apply(input), camel, "camel({input})");
            assert_eq!(
                CaseTransform::Pascal.apply(input),
                pascal,
                "pascal({input})"
            );
            assert_eq!(CaseTransform::Upper.apply(input), upper, "upper({input})");
            assert_eq!(CaseTransform::Kebab.apply(input), kebab, "kebab({input})");
            assert_eq!(CaseTransform::Lower.apply(input), lower, "lower({input})");
        }
    }

    #[test]
    fn transforms_are_idempotent_over_the_table() {
        let all = [
            CaseTransform::Snake,
            CaseTransform::Camel,
            CaseTransform::Pascal,
            CaseTransform::Upper,
            CaseTransform::Kebab,
            CaseTransform::Lower,
        ];
        for &(input, ..) in CASES {
            for t in all {
                // Known degenerate case, pinned below: pascal/camel join
                // single-letter tokens into an acronym run whose boundaries
                // cannot be recovered (`a b` → `AB` → `Ab`).
                if input == "a b" && matches!(t, CaseTransform::Pascal | CaseTransform::Camel) {
                    continue;
                }
                let once = t.apply(input);
                assert_eq!(t.apply(&once), once, "{}({input}) not idempotent", t.name());
            }
        }
    }

    #[test]
    fn pascal_merging_single_letter_tokens_is_lossy() {
        // Joining without a separator erases the boundary; the re-tokenizer
        // then reads `AB` as one acronym. Documented, not fixable without
        // remembering the original boundaries.
        assert_eq!(CaseTransform::Pascal.apply("a b"), "AB");
        assert_eq!(CaseTransform::Pascal.apply("AB"), "Ab");
    }

    #[test]
    fn convention_transforms_agree_across_input_spellings() {
        // snake/pascal/camel/upper/kebab reach the same output no matter
        // which convention the input already uses.
        for input in ["maxSize", "max_size", "MaxSize", "MAX_SIZE", "max-size"] {
            assert_eq!(CaseTransform::Snake.apply(input), "max_size");
            assert_eq!(CaseTransform::Pascal.apply(input), "MaxSize");
            assert_eq!(CaseTransform::Camel.apply(input), "maxSize");
            assert_eq!(CaseTransform::Upper.apply(input), "MAX_SIZE");
            assert_eq!(CaseTransform::Kebab.apply(input), "max-size");
        }
    }

    #[test]
    fn all_underscores_and_separator_noise() {
        assert_eq!(CaseTransform::Snake.apply("___"), "___");
        assert_eq!(CaseTransform::Snake.apply("a__b"), "a_b");
        assert_eq!(CaseTransform::Pascal.apply("-a- -b-"), "AB");
        assert_eq!(CaseTransform::Snake.apply("trailing_"), "trailing");
    }
}
