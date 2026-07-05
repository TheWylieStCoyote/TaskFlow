//! Wrap-with-abbreviation: splice a selection into an expansion's `$0` slot.
//!
//! Shared by the LSP `codemmet.wrap` command and the `codemmet wrap`
//! subcommand. The expansion is always rendered in keep mode (LSP snippet
//! syntax); [`strip_snippet_markers`] converts to plain text afterwards.

use codemmet_core::Engine;
use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::ExpandError;
use codemmet_core::snippet::render::TabstopMode;

/// Why a wrap request failed.
#[derive(Debug)]
pub enum WrapError {
    /// The abbreviation failed to parse or render.
    Expand(ExpandError),
    /// The rendered expansion carries no `$0` to splice into (defensive;
    /// the renderer guarantees exactly one).
    NoCursor,
}

impl std::fmt::Display for WrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expand(err) => err.fmt(f),
            Self::NoCursor => write!(f, "expansion has no final cursor to wrap at"),
        }
    }
}

impl std::error::Error for WrapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Expand(err) => Some(err),
            Self::NoCursor => None,
        }
    }
}

impl From<ExpandError> for WrapError {
    fn from(err: ExpandError) -> Self {
        Self::Expand(err)
    }
}

/// Expand `abbr` in keep mode and splice `selection` in at the `$0` slot.
///
/// The selection is dedented to depth 0, escaped for snippet syntax, and
/// re-indented with the `$0` line's leading whitespace: the first selection
/// line continues at the `$0` position, subsequent lines carry that prefix.
/// `$0` lands immediately after the injected text.
pub fn wrap(
    engine: &Engine,
    language: &str,
    abbr: &str,
    selection: &str,
    ctx: &ExpansionContext,
) -> Result<String, WrapError> {
    let expansion = engine.expand(abbr, language, ctx, TabstopMode::Keep)?;
    let body = expansion.body;
    let slot = find_unescaped(&body, "$0").ok_or(WrapError::NoCursor)?;

    let line_start = body[..slot].rfind('\n').map_or(0, |i| i + 1);
    let hang: String = body[line_start..]
        .chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect();

    let escaped = escape_snippet_text(&dedent(selection));
    let mut injected = String::new();
    for (i, line) in escaped.split('\n').enumerate() {
        if i > 0 {
            injected.push('\n');
            if !line.is_empty() {
                injected.push_str(&hang);
            }
        }
        injected.push_str(line);
    }

    let mut out = String::with_capacity(body.len() + injected.len());
    out.push_str(&body[..slot]);
    out.push_str(&injected);
    out.push_str("$0");
    out.push_str(&body[slot + "$0".len()..]);
    Ok(out)
}

/// First occurrence of `needle` not preceded by an odd run of backslashes.
fn find_unescaped(text: &str, needle: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(rel) = text[from..].find(needle) {
        let at = from + rel;
        let backslashes = bytes[..at]
            .iter()
            .rev()
            .take_while(|&&b| b == b'\\')
            .count();
        if backslashes % 2 == 0 {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// Strip the common leading whitespace of `text`'s non-empty lines.
fn dedent(text: &str) -> String {
    let common = common_indent(text);
    let lines: Vec<&str> = text
        .split('\n')
        .map(|line| line.strip_prefix(common).unwrap_or(line))
        .collect();
    lines.join("\n")
}

/// The longest whitespace prefix shared by every non-blank line.
fn common_indent(text: &str) -> &str {
    let mut common: Option<&str> = None;
    for line in text.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let stripped = line.trim_start_matches([' ', '\t']);
        let indent = &line[..line.len() - stripped.len()];
        common = Some(common.map_or(indent, |prev| shared_prefix(prev, indent)));
    }
    common.unwrap_or("")
}

/// The longest common prefix of two strings (both are pure whitespace here).
fn shared_prefix<'a>(a: &'a str, b: &str) -> &'a str {
    let len = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    &a[..len]
}

/// Escape literal text for inclusion in LSP snippet syntax.
fn escape_snippet_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '$' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Convert LSP snippet syntax to plain text: unescape `\$`/`\}`/`\\`, replace
/// `${n:placeholder}` with its placeholder, and drop `$n`/`${n}`/`$0`.
///
/// This is a full walker over the snippet grammar our renderer emits (no
/// nested placeholders), not a regex.
pub fn strip_snippet_markers(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        match c {
            '\\' => {
                if let Some(escaped) = rest[1..].chars().next() {
                    if !matches!(escaped, '$' | '}' | '\\') {
                        // Not a snippet escape: the backslash is literal.
                        out.push('\\');
                    }
                    out.push(escaped);
                    rest = &rest[1 + escaped.len_utf8()..];
                } else {
                    out.push('\\');
                    rest = "";
                }
            }
            '$' => {
                if let Some((placeholder, consumed)) = parse_marker(rest) {
                    if let Some(placeholder) = placeholder {
                        out.push_str(&strip_snippet_markers(placeholder));
                    }
                    rest = &rest[consumed..];
                } else {
                    out.push('$');
                    rest = &rest[1..];
                }
            }
            c => {
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

/// Parse a tabstop marker at the start of `text` (which begins with `$`).
///
/// Returns the raw placeholder text (for `${n:...}`) and the marker's byte
/// length, or `None` when `$` is not followed by a marker.
fn parse_marker(text: &str) -> Option<(Option<&str>, usize)> {
    let rest = &text[1..];
    if let Some(inner) = rest.strip_prefix('{') {
        let digits = inner.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        let after = &inner[digits..];
        // `${n}`: no placeholder.
        if let Some(stripped) = after.strip_prefix('}') {
            let consumed = text.len() - stripped.len();
            return Some((None, consumed));
        }
        // `${n:placeholder}`: scan to the unescaped closing brace.
        let placeholder = after.strip_prefix(':')?;
        let close = find_unescaped(placeholder, "}")?;
        let consumed = text.len() - placeholder.len() + close + 1;
        Some((Some(&placeholder[..close]), consumed))
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        (digits > 0).then_some((None, 1 + digits))
    }
}

#[cfg(test)]
// Snippet bodies like `${1:i}` are fixture data, not format strings.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use codemmet_core::context::IndentUnit;
    use codemmet_core::lang::registry::Registry;

    use super::*;

    fn engine() -> Engine {
        Engine::with_registry(Registry::with_builtins().expect("builtins should load"))
    }

    fn ctx(base_indent: &str) -> ExpansionContext {
        ExpansionContext::plain(base_indent.to_owned(), IndentUnit::Spaces(4))
    }

    fn wrap_ok(abbr: &str, selection: &str, base_indent: &str) -> String {
        wrap(&engine(), "rust", abbr, selection, &ctx(base_indent))
            .expect("wrap fixture should succeed")
    }

    #[test]
    fn single_line_selection_lands_at_cursor_with_trailing_final() {
        let got = wrap_ok("fori10", "println!(\"x\");", "");
        assert_eq!(got, "for ${1:i} in 0..10 {\n    println!(\"x\");$0\n}");
    }

    #[test]
    fn multi_line_selection_is_dedented_and_reindented() {
        let got = wrap_ok("fori10", "    a();\n        b();\n    c();", "");
        assert_eq!(
            got,
            "for ${1:i} in 0..10 {\n    a();\n        b();\n    c();$0\n}"
        );
    }

    #[test]
    fn blank_selection_lines_stay_blank() {
        let got = wrap_ok("fori10", "a();\n\nb();", "");
        assert_eq!(got, "for ${1:i} in 0..10 {\n    a();\n\n    b();$0\n}");
    }

    #[test]
    fn selection_dollars_and_backslashes_are_escaped() {
        let got = wrap_ok("fori10", "let s = \"$HOME\\\\\";", "");
        assert_eq!(
            got,
            "for ${1:i} in 0..10 {\n    let s = \"\\$HOME\\\\\\\\\";$0\n}"
        );
    }

    #[test]
    fn base_indent_flows_into_the_hanging_indent() {
        let got = wrap_ok("fori10", "a();\nb();", "    ");
        assert_eq!(
            got,
            "for ${1:i} in 0..10 {\n        a();\n        b();$0\n    }"
        );
    }

    #[test]
    fn multi_line_selection_in_inline_construct() {
        // `pr` puts `$0` mid-line inside the quotes; the second selection
        // line hangs at the line's leading whitespace (none here).
        let got = wrap_ok("pr", "a\nb", "");
        assert_eq!(got, "println!(\"a\nb$0\");");
    }

    #[test]
    fn wrap_error_display_for_unknown_trigger() {
        let err = wrap(&engine(), "rust", "zzzz", "x", &ctx(""))
            .expect_err("unknown trigger should fail");
        assert!(matches!(err, WrapError::Expand(ExpandError::Render(_))));
    }

    #[test]
    fn find_unescaped_skips_escaped_occurrences() {
        assert_eq!(find_unescaped("\\$0 $0", "$0"), Some(4));
        assert_eq!(find_unescaped("\\\\$0", "$0"), Some(2));
        assert_eq!(find_unescaped("\\$0", "$0"), None);
    }

    #[test]
    fn dedent_strips_common_whitespace_only() {
        assert_eq!(dedent("    a\n      b\n    c"), "a\n  b\nc");
        assert_eq!(dedent("\ta\n\t\tb"), "a\n\tb");
        assert_eq!(dedent("a\n    b"), "a\n    b");
        assert_eq!(dedent("    a\n\n    b"), "a\n\nb");
    }

    #[test]
    fn strip_walker_handles_all_marker_shapes() {
        assert_eq!(
            strip_snippet_markers("for ${1:i} in 0..${2:10} {$0}"),
            "for i in 0..10 {}"
        );
        assert_eq!(strip_snippet_markers("a $1 b ${2} c"), "a  b  c");
        assert_eq!(
            strip_snippet_markers("\\$HOME and \\\\ and \\}"),
            "$HOME and \\ and }"
        );
        assert_eq!(strip_snippet_markers("${1:a\\}b}"), "a}b");
        // Non-marker dollars survive.
        assert_eq!(strip_snippet_markers("$x ${y} $"), "$x ${y} $");
    }

    #[test]
    fn wrap_then_strip_round_trip() {
        let kept = wrap_ok("fori10", "println!(\"$x\");", "");
        assert_eq!(
            strip_snippet_markers(&kept),
            "for i in 0..10 {\n    println!(\"$x\");\n}"
        );
    }
}
