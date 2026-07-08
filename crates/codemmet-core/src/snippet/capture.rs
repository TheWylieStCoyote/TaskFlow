//! Capture-from-example: turn a snippet of real code into a reusable,
//! parameterized snippet body.
//!
//! This is the inverse of expansion. Where [`render`](crate::snippet::render)
//! takes a template and produces code, [`capture`] takes code and produces a
//! template: it generalizes the *varying* identifiers into named `${param}`
//! references so the same shape can be stamped out again with a different name.
//!
//! # Generalization heuristic
//!
//! With only one example there is no diff to learn from, so the heuristic
//! guesses which identifiers are "the thing being varied" and is deliberately
//! simple and predictable:
//!
//! - The code is scanned for identifier tokens (`[A-Za-z_][A-Za-z0-9_]*`).
//! - An identifier becomes a **parameter** when it occurs **more than once**
//!   and is not a language keyword — a repeated identifier is almost always
//!   the subject of the pattern (a type name echoed across a signature).
//!   Repeated occurrences render as a mirror, so renaming one renames all.
//! - Every other identifier, and all punctuation/keywords, stay **literal**.
//! - `force_params` promotes an identifier to a parameter even when it occurs
//!   once or is a keyword; `force_literals` keeps one literal even when it
//!   repeats. `force_literals` wins ties.
//!
//! Matching is whole-token: `foo` and `handle_foo` are different identifiers,
//! so parameterizing `foo` does not touch `handle_foo`. This keeps the result
//! obvious at the cost of missing substring-level variation — the caller can
//! always hand-edit the generated TOML afterwards.
//!
//! The captured body is dedented (its common leading whitespace removed) and
//! re-indented to tabs, so it satisfies the template indentation rules and
//! adapts to any target indent unit at expansion time. Literal `$` and `\`
//! are escaped, so the body round-trips: capturing code and then expanding
//! the trigger with no arguments reproduces the original (dedented) text.

use crate::config::schema::{ParamDef, ParamKind};

/// A snippet generalized from example code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    /// The declared parameters, in first-occurrence order.
    pub params: Vec<ParamDef>,
    /// The template body (tab-indented, with `${param}` references and escaped
    /// literals). Has no trailing newline.
    pub body: String,
}

/// Options controlling which identifiers [`capture`] generalizes.
#[derive(Debug, Default, Clone)]
pub struct CaptureOptions {
    /// Identifiers to force into parameters even if they occur once.
    pub force_params: Vec<String>,
    /// Identifiers to keep literal even if they repeat.
    pub force_literals: Vec<String>,
}

/// Generalize `code` into a parameterized snippet for `language`.
///
/// `language` is the canonical profile id (for keyword filtering); an unknown
/// id simply means no keywords are filtered.
#[must_use]
pub fn capture(language: &str, code: &str, options: &CaptureOptions) -> Captured {
    let body_src = normalize_indent(code);
    let tokens = tokenize(&body_src);

    // Frequency of each identifier token.
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for token in &tokens {
        if let Token::Ident(name) = token {
            *counts.entry(name).or_default() += 1;
        }
    }

    let keywords = keywords_for(language);
    let is_param = |name: &str| -> bool {
        if options.force_literals.iter().any(|l| l == name) {
            return false;
        }
        if options.force_params.iter().any(|p| p == name) {
            return true;
        }
        counts.get(name).copied().unwrap_or(0) >= 2 && !keywords.contains(&name)
    };

    // Assign a parameter name per identifier, in first-occurrence order.
    let mut params: Vec<ParamDef> = Vec::new();
    let mut param_name_of: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for token in &tokens {
        if let Token::Ident(name) = token
            && is_param(name)
            && !param_name_of.contains_key(*name)
        {
            let param_name = unique_param_name(name, &params);
            params.push(ParamDef {
                name: param_name.clone(),
                kind: ParamKind::Ident,
                default: Some((*name).to_owned()),
            });
            param_name_of.insert((*name).to_owned(), param_name);
        }
    }

    // Emit the body: parameters become `${name}`, everything else is escaped
    // literal text.
    let mut body = String::with_capacity(body_src.len());
    for token in &tokens {
        match token {
            Token::Ident(name) => {
                if let Some(param_name) = param_name_of.get(*name) {
                    body.push_str("${");
                    body.push_str(param_name);
                    body.push('}');
                } else {
                    body.push_str(name);
                }
            }
            Token::Other(text) => push_escaped(&mut body, text),
        }
    }

    Captured { params, body }
}

/// A lexical run of the source: an identifier or a stretch of everything else.
enum Token<'a> {
    /// A `[A-Za-z_][A-Za-z0-9_]*` identifier.
    Ident(&'a str),
    /// Any other run of characters (punctuation, whitespace, numbers, …).
    Other(&'a str),
}

/// Split `src` into identifier and non-identifier runs, in order.
fn tokenize(src: &str) -> Vec<Token<'_>> {
    let bytes = src.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if is_ident_start(bytes[i]) {
            i += 1;
            while i < bytes.len() && is_ident_continue(bytes[i]) {
                i += 1;
            }
            tokens.push(Token::Ident(&src[start..i]));
        } else {
            i += 1;
            while i < bytes.len() && !is_ident_start(bytes[i]) {
                i += 1;
            }
            tokens.push(Token::Other(&src[start..i]));
        }
    }
    tokens
}

/// Whether `b` can start an identifier.
const fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

/// Whether `b` can continue an identifier.
const fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Append `text` to `body`, escaping the template metacharacters `$` and `\`.
///
/// A literal `$` becomes `\$` and a literal `\` becomes `\\`; in strip mode
/// the renderer turns these back into `$` and `\`, so the source round-trips.
/// `{`/`}` need no escaping on their own — only `${` is special, and escaping
/// every `$` already defuses it.
fn push_escaped(body: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '$' => body.push_str("\\$"),
            '\\' => body.push_str("\\\\"),
            _ => body.push(c),
        }
    }
}

/// A parameter name for identifier `ident` that is valid, not reserved, and
/// unique among `existing`.
///
/// Identifiers are already valid WORDs, so the name is the identifier itself
/// unless it collides with a reserved name (`i`, `value`, `outer`) or a name
/// already taken, in which case a numeric suffix is appended.
fn unique_param_name(ident: &str, existing: &[ParamDef]) -> String {
    let taken = |name: &str| {
        name == "i" || name == "value" || name == "outer" || existing.iter().any(|p| p.name == name)
    };
    if !taken(ident) {
        return ident.to_owned();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{ident}{n}");
        if !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Dedent `code` and convert its indentation to tabs (one tab per level).
///
/// The common leading whitespace of the non-blank lines is stripped, then the
/// remaining indentation is measured in the document's own unit (a leading tab
/// makes it tab-based; otherwise the smallest positive space run is one level)
/// and rewritten as that many tabs. The result satisfies the template
/// indentation rules regardless of how the source was indented. A trailing
/// newline is trimmed.
fn normalize_indent(code: &str) -> String {
    let code = code.trim_end_matches(['\r', '\n']);
    let dedented = dedent(code);
    let unit = detect_indent_width(&dedented);
    let mut out = String::with_capacity(dedented.len());
    for (idx, line) in dedented.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        let content = line.trim_start_matches([' ', '\t']);
        let indent = &line[..line.len() - content.len()];
        let depth = indent_depth(indent, unit);
        for _ in 0..depth {
            out.push('\t');
        }
        out.push_str(content);
    }
    out
}

/// The number of indent levels `indent` represents, given the space `unit`.
///
/// A leading tab counts each tab as one level; otherwise the space count is
/// divided by `unit` (integer division tolerates ragged indentation).
fn indent_depth(indent: &str, unit: usize) -> usize {
    if indent.starts_with('\t') {
        indent.bytes().filter(|&b| b == b'\t').count()
    } else if unit == 0 {
        0
    } else {
        indent.len() / unit
    }
}

/// The document's indent unit as a space width, or 0 for tab/flat documents.
///
/// A leading tab anywhere makes the document tab-indented (width reported as
/// 0, which [`indent_depth`] handles by counting tabs). Otherwise it is the
/// smallest positive leading-space run seen.
fn detect_indent_width(text: &str) -> usize {
    let mut smallest: Option<usize> = None;
    for line in text.split('\n') {
        let content = line.trim_start_matches([' ', '\t']);
        if content.is_empty() {
            continue;
        }
        let indent = &line[..line.len() - content.len()];
        if indent.starts_with('\t') {
            return 0;
        }
        if !indent.is_empty() {
            smallest = Some(smallest.map_or(indent.len(), |n| n.min(indent.len())));
        }
    }
    smallest.unwrap_or(0)
}

/// Strip the common leading whitespace of `text`'s non-blank lines.
///
/// The common prefix is literal (tabs and spaces are not width-equated); blank
/// lines do not contribute to it.
fn dedent(text: &str) -> String {
    let mut common: Option<&str> = None;
    for line in text.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let stripped = line.trim_start_matches([' ', '\t']);
        let indent = &line[..line.len() - stripped.len()];
        common = Some(common.map_or(indent, |prev| {
            let len = prev
                .bytes()
                .zip(indent.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            &prev[..len]
        }));
    }
    let common = common.unwrap_or("");
    text.split('\n')
        .map(|line| line.strip_prefix(common).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A modest set of keywords per language, so obviously-structural words are
/// never generalized just because they repeat.
///
/// Not exhaustive: a keyword that slips through can be pinned with
/// `force_literals`, and one that is over-filtered with `force_params`.
#[allow(clippy::too_many_lines)]
fn keywords_for(language: &str) -> &'static [&'static str] {
    match language {
        "rust" => &[
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
            "trait", "true", "type", "unsafe", "use", "where", "while",
        ],
        "python" => &[
            "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
            "elif", "else", "except", "False", "finally", "for", "from", "global", "if", "import",
            "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return",
            "self", "True", "try", "while", "with", "yield",
        ],
        "c" | "cpp" => &[
            "auto",
            "break",
            "case",
            "char",
            "class",
            "const",
            "continue",
            "default",
            "do",
            "double",
            "else",
            "enum",
            "extern",
            "false",
            "float",
            "for",
            "goto",
            "if",
            "int",
            "long",
            "namespace",
            "new",
            "private",
            "protected",
            "public",
            "return",
            "short",
            "signed",
            "sizeof",
            "static",
            "struct",
            "switch",
            "template",
            "this",
            "true",
            "typedef",
            "union",
            "unsigned",
            "using",
            "virtual",
            "void",
            "volatile",
            "while",
        ],
        "javascript" | "typescript" => &[
            "async",
            "await",
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "default",
            "delete",
            "do",
            "else",
            "enum",
            "export",
            "extends",
            "false",
            "finally",
            "for",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "interface",
            "let",
            "new",
            "null",
            "return",
            "super",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "type",
            "typeof",
            "var",
            "void",
            "while",
            "yield",
        ],
        "go" => &[
            "break",
            "case",
            "chan",
            "const",
            "continue",
            "default",
            "defer",
            "else",
            "fallthrough",
            "for",
            "func",
            "go",
            "goto",
            "if",
            "import",
            "interface",
            "map",
            "package",
            "range",
            "return",
            "select",
            "struct",
            "switch",
            "type",
            "var",
        ],
        _ => &[],
    }
}

#[cfg(test)]
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::*;

    fn opts() -> CaptureOptions {
        CaptureOptions::default()
    }

    #[test]
    fn repeated_identifier_becomes_a_mirrored_param() {
        // The standalone `Foo` token occurs twice, so it is generalized into a
        // mirrored parameter. `make_Foo` is a single distinct token (whole-token
        // matching), so it is NOT touched — the documented limitation.
        let code = "fn make_Foo() -> Foo {\n    Foo::new()\n}";
        let captured = capture("rust", code, &opts());
        assert_eq!(captured.params.len(), 1);
        assert_eq!(captured.params[0].name, "Foo");
        assert_eq!(captured.params[0].default.as_deref(), Some("Foo"));
        assert_eq!(captured.params[0].kind, ParamKind::Ident);
        assert_eq!(
            captured.body,
            "fn make_Foo() -> ${Foo} {\n\t${Foo}::new()\n}"
        );
    }

    #[test]
    fn single_occurrence_identifiers_stay_literal() {
        let code = "let x = compute();";
        let captured = capture("rust", code, &opts());
        assert!(captured.params.is_empty());
        assert_eq!(captured.body, "let x = compute();");
    }

    #[test]
    fn force_param_promotes_a_single_occurrence_identifier() {
        let code = "let x = compute();";
        let captured = capture(
            "rust",
            code,
            &CaptureOptions {
                force_params: vec!["compute".to_owned()],
                ..CaptureOptions::default()
            },
        );
        assert_eq!(captured.params.len(), 1);
        assert_eq!(captured.params[0].name, "compute");
        assert_eq!(captured.body, "let x = ${compute}();");
    }

    #[test]
    fn force_literal_keeps_a_repeated_identifier_literal() {
        let code = "foo(); foo();";
        let captured = capture(
            "rust",
            code,
            &CaptureOptions {
                force_literals: vec!["foo".to_owned()],
                ..CaptureOptions::default()
            },
        );
        assert!(captured.params.is_empty());
        assert_eq!(captured.body, "foo(); foo();");
    }

    #[test]
    fn keywords_are_not_generalized_even_when_repeated() {
        // `let` repeats but is a keyword; nothing is generalized.
        let code = "let a = 1;\nlet b = 2;";
        let captured = capture("rust", code, &opts());
        assert!(captured.params.is_empty(), "params: {:?}", captured.params);
    }

    #[test]
    fn indentation_is_dedented_and_tabbed() {
        // Four-space indentation, wrapped in a common leading indent.
        let code = "        for x in xs {\n            work(x);\n        }";
        let captured = capture("rust", code, &opts());
        // `x` and `xs` occur; `x` twice → param, `xs` once → literal.
        assert_eq!(captured.params.len(), 1);
        assert_eq!(captured.params[0].name, "x");
        assert_eq!(captured.body, "for ${x} in xs {\n\twork(${x});\n}");
    }

    #[test]
    fn tab_indentation_is_preserved_as_depth() {
        let code = "func f() {\n\treturn f()\n}";
        let captured = capture("go", code, &opts());
        // `f` occurs twice → param.
        assert_eq!(captured.params[0].name, "f");
        assert_eq!(captured.body, "func ${f}() {\n\treturn ${f}()\n}");
    }

    #[test]
    fn dollars_and_backslashes_are_escaped() {
        let code = "let s = \"$var\\n\";";
        let captured = capture("rust", code, &opts());
        assert!(captured.params.is_empty());
        assert_eq!(captured.body, "let s = \"\\$var\\\\n\";");
    }

    #[test]
    fn reserved_names_get_a_suffix() {
        // An identifier literally named `value` repeats; it must not be used as
        // a parameter name (it is reserved), so it is suffixed.
        let code = "value + value";
        let captured = capture("rust", code, &opts());
        assert_eq!(captured.params[0].name, "value2");
        assert_eq!(captured.body, "${value2} + ${value2}");
    }
}
