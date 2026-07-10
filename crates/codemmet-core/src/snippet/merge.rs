//! Merging per-expansion keep-mode snippet bodies into one snippet document.
//!
//! File mode expands each `.codemmet` line independently, so every line's
//! keep-mode body restarts tabstop numbering at `$1` and carries its own
//! `$0` — the plain concatenation is not one valid LSP snippet. This module
//! rebuilds validity by applying [`renumber`](super::render)'s single-snippet
//! rules across the whole file, treating each line's expansion exactly like a
//! sibling group inside one abbreviation (`if+el`):
//!
//! - Tabstop identities are **line-local**: fresh global numbers are assigned
//!   by first occurrence in document order, so mirrors within a line keep
//!   their linkage and lines never alias each other's stops.
//! - Every `$0` candidate except the **last in document order** demotes to a
//!   plain numbered stop at its position (the `if+el`/`arm*3` precedent);
//!   exactly one final cursor survives — the `$0` of the last line that has
//!   one.
//!
//! The input bodies are expected to be this crate's own keep-mode renderer
//! output. Parsing is lenient: only `$n`, `${n}`, `${n:placeholder}`, and
//! `$0` tokens are rewritten, escapes (`\$`, `\\`, `\}`) pass through
//! verbatim, and anything unrecognized is left as literal text — so escaping
//! survives concatenation byte-for-byte.

use std::collections::HashMap;
use std::fmt::Write as _;

/// Accumulates keep-mode expansion bodies and blank lines in file order,
/// then renumbers them into one valid snippet document.
///
/// Mirrors the strip-mode buffer discipline of file mode: each block is one
/// abbreviation line's expansion (its base indent prefixing the first body
/// line), and each blank is one blank output line.
#[derive(Debug, Default)]
pub struct SnippetDocBuilder {
    parts: Vec<Part>,
}

/// One file-order piece of the document.
#[derive(Debug)]
enum Part {
    /// A blank output line.
    Blank,
    /// One expansion's tokenized keep-mode body.
    Block(Vec<Segment>),
}

/// One token of a keep-mode body.
#[derive(Debug)]
enum Segment {
    /// Literal snippet text, kept exactly as escaped by the renderer.
    Text(String),
    /// A numbered tabstop occurrence (`$n`, `${n}`, or `${n:placeholder}`).
    Stop {
        /// The line-local tabstop number.
        local: u32,
        /// The raw (still-escaped) placeholder text, for `${n:…}` forms.
        placeholder: Option<String>,
    },
    /// A `$0` final-cursor candidate; only the last one in the document
    /// survives as `$0`.
    Final {
        /// The raw placeholder text, for a (nonstandard) `${0:…}` form.
        placeholder: Option<String>,
    },
}

impl SnippetDocBuilder {
    /// A builder with no content yet.
    pub const fn new() -> Self {
        Self { parts: Vec::new() }
    }

    /// Append one blank output line.
    pub fn push_blank(&mut self) {
        self.parts.push(Part::Blank);
    }

    /// Append one expansion's keep-mode `body`, its first line prefixed by
    /// `indent` (continuation lines already carry the base indent — the
    /// renderer threads it through `ExpansionContext::base_indent`).
    pub fn push_block(&mut self, indent: &str, body: &str) {
        let mut segments = Vec::new();
        if !indent.is_empty() {
            segments.push(Segment::Text(indent.to_string()));
        }
        segments.extend(tokenize(body));
        self.parts.push(Part::Block(segments));
    }

    /// Render the merged document: fresh tabstop numbers in document order,
    /// exactly one surviving `$0` (the last candidate), every other
    /// candidate demoted to a numbered stop. Non-empty output always ends
    /// with a newline, matching strip-mode file output.
    pub fn finish(self) -> String {
        let last_final = self.last_final_position();
        let mut numbers: HashMap<(usize, u32), u32> = HashMap::new();
        let mut next: u32 = 1;
        let mut out = String::new();
        for (block, part) in self.parts.iter().enumerate() {
            let Part::Block(segments) = part else {
                out.push('\n');
                continue;
            };
            for (index, segment) in segments.iter().enumerate() {
                match segment {
                    Segment::Text(text) => out.push_str(text),
                    Segment::Stop { local, placeholder } => {
                        let number = *numbers.entry((block, *local)).or_insert_with(|| {
                            let n = next;
                            next += 1;
                            n
                        });
                        write_stop(&mut out, number, placeholder.as_deref());
                    }
                    Segment::Final { placeholder } => {
                        let number = if last_final == Some((block, index)) {
                            0
                        } else {
                            let n = next;
                            next += 1;
                            n
                        };
                        write_stop(&mut out, number, placeholder.as_deref());
                    }
                }
            }
            out.push('\n');
        }
        out
    }

    /// The `(part, segment)` position of the last `$0` candidate, if any.
    fn last_final_position(&self) -> Option<(usize, usize)> {
        self.parts
            .iter()
            .enumerate()
            .flat_map(|(block, part)| {
                let segments = match part {
                    Part::Block(segments) => segments.as_slice(),
                    Part::Blank => &[],
                };
                segments
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| matches!(s, Segment::Final { .. }))
                    .map(move |(index, _)| (block, index))
            })
            .last()
    }
}

/// Write one tabstop occurrence: `${n:placeholder}` when it carries a
/// placeholder, bare `$n` otherwise (`$0` for the surviving final).
fn write_stop(out: &mut String, number: u32, placeholder: Option<&str>) {
    match placeholder {
        Some(text) => {
            let _ = write!(out, "${{{number}:{text}}}");
        }
        None => {
            let _ = write!(out, "${number}");
        }
    }
}

/// Split a keep-mode body into literal text and tabstop tokens.
///
/// Escapes pass through verbatim inside [`Segment::Text`]; a `$` that does
/// not start a recognizable tabstop is literal text too.
fn tokenize(body: &str) -> Vec<Segment> {
    let bytes = body.as_bytes();
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let run = bytes[i..]
            .iter()
            .position(|&b| b == b'$' || b == b'\\')
            .unwrap_or(bytes.len() - i);
        text.push_str(&body[i..i + run]);
        i += run;
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'\\' {
            // An escape passes through verbatim with its escaped character.
            let end = body[i + 1..]
                .chars()
                .next()
                .map_or(i + 1, |c| i + 1 + c.len_utf8());
            text.push_str(&body[i..end]);
            i = end;
        } else if let Some((segment, len)) = parse_stop(&body[i..]) {
            if !text.is_empty() {
                segments.push(Segment::Text(std::mem::take(&mut text)));
            }
            segments.push(segment);
            i += len;
        } else {
            text.push('$');
            i += 1;
        }
    }
    if !text.is_empty() {
        segments.push(Segment::Text(text));
    }
    segments
}

/// Parse a tabstop token at the start of `s` (which begins with `$`):
/// `$n`, `${n}`, or `${n:placeholder}`. Returns the segment and the token's
/// byte length, or `None` when `$` is not a tabstop here.
fn parse_stop(s: &str) -> Option<(Segment, usize)> {
    let rest = &s[1..];
    if let Some(braced) = rest.strip_prefix('{') {
        let digits = leading_digits(braced)?;
        let number: u32 = braced[..digits].parse().ok()?;
        return match braced.as_bytes().get(digits) {
            Some(b'}') => Some((stop(number, None), 2 + digits + 1)),
            Some(b':') => {
                let placeholder_start = digits + 1;
                let close = unescaped_close(&braced[placeholder_start..])?;
                let placeholder = braced[placeholder_start..placeholder_start + close].to_string();
                Some((
                    stop(number, Some(placeholder)),
                    2 + placeholder_start + close + 1,
                ))
            }
            _ => None,
        };
    }
    let digits = leading_digits(rest)?;
    let number: u32 = rest[..digits].parse().ok()?;
    Some((stop(number, None), 1 + digits))
}

/// The byte length of the leading ASCII-digit run, if there is one.
fn leading_digits(s: &str) -> Option<usize> {
    let len = s.bytes().take_while(u8::is_ascii_digit).count();
    (len > 0).then_some(len)
}

/// The byte offset of the first unescaped `}` in `s`, if any.
///
/// Scanning is byte-wise: `}` is ASCII, so in valid UTF-8 every match sits
/// on a character boundary, and a `\`-escaped multibyte character contains
/// no `}` bytes to trip over.
fn unescaped_close(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => i += 2,
            b'}' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

/// A [`Segment`] for a parsed number: `0` is a final-cursor candidate.
const fn stop(number: u32, placeholder: Option<String>) -> Segment {
    if number == 0 {
        Segment::Final { placeholder }
    } else {
        Segment::Stop {
            local: number,
            placeholder,
        }
    }
}

#[cfg(test)]
// Snippet bodies like `${1:pattern}` are fixture data, not format strings.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::*;

    /// Build a document from `(indent, body)` blocks with no blanks.
    fn merge(blocks: &[(&str, &str)]) -> String {
        let mut doc = SnippetDocBuilder::new();
        for (indent, body) in blocks {
            doc.push_block(indent, body);
        }
        doc.finish()
    }

    #[test]
    fn empty_builder_renders_nothing() {
        assert_eq!(SnippetDocBuilder::new().finish(), "");
    }

    #[test]
    fn single_block_passes_through() {
        assert_eq!(
            merge(&[("", "fn main($1) {\n    $0\n}")]),
            "fn main($1) {\n    $0\n}\n"
        );
    }

    #[test]
    fn stops_renumber_across_blocks_and_earlier_finals_demote() {
        // The `if+el` sibling rule, applied across lines: line 1's `$0`
        // becomes the next numbered stop, line 2's numbering follows on.
        assert_eq!(
            merge(&[("", "fn main($1) {\n    $0\n}"), ("", "println!(\"$0\");")]),
            "fn main($1) {\n    $2\n}\nprintln!(\"$0\");\n"
        );
    }

    #[test]
    fn mirrors_stay_linked_within_their_own_line_only() {
        // Both lines use local `$1` with a mirror; each line's pair maps to
        // one shared global number, and the lines never alias each other.
        assert_eq!(
            merge(&[
                ("", "impl ${1:T} {\n    $1\n}$0"),
                ("", "let ${1:x} = $1;$0")
            ]),
            "impl ${1:T} {\n    $1\n}$2\nlet ${3:x} = $3;$0\n"
        );
    }

    #[test]
    fn surviving_final_is_the_last_lines_even_mid_document() {
        // Only the first line carries a `$0`: it survives (it is the last
        // candidate in document order), and later stops number past it.
        assert_eq!(merge(&[("", "$1 $0"), ("", "${1:x}")]), "$1 $0\n${2:x}\n");
    }

    #[test]
    fn demoted_final_takes_its_number_in_document_order() {
        // A `$0` before a line's later stops numbers ahead of them, exactly
        // like the single-snippet renumbering pass.
        assert_eq!(merge(&[("", "a $0 b $1"), ("", "$0")]), "a $1 b $2\n$0\n");
    }

    #[test]
    fn blank_lines_pass_through_between_blocks() {
        let mut doc = SnippetDocBuilder::new();
        doc.push_block("", "$1$0");
        doc.push_blank();
        doc.push_blank();
        doc.push_block("", "$1$0");
        assert_eq!(doc.finish(), "$1$2\n\n\n$3$0\n");
    }

    #[test]
    fn indent_prefixes_only_the_first_line() {
        assert_eq!(
            merge(&[("    ", "for $1 {\n    x$0\n}")]),
            "    for $1 {\n    x$0\n}\n"
        );
    }

    #[test]
    fn escapes_pass_through_verbatim() {
        // `\$` and `\\` in text, `\}` in a placeholder: byte-for-byte, with
        // only the tabstop numbers rewritten.
        assert_eq!(
            merge(&[("", "cost \\$5 or \\\\$0"), ("", "${1:a\\}b}$0")]),
            "cost \\$5 or \\\\$1\n${2:a\\}b}$0\n"
        );
    }

    #[test]
    fn braced_bare_stop_normalizes_to_bare_form() {
        assert_eq!(merge(&[("", "${1}$0"), ("", "${1}$0")]), "$1$2\n$3$0\n");
    }

    #[test]
    fn unrecognized_dollar_forms_stay_literal() {
        // No digits, unterminated placeholder, trailing `$`: all literal.
        assert_eq!(merge(&[("", "$x ${y} ${1:open $")]), "$x ${y} ${1:open $\n");
    }

    #[test]
    fn blocks_without_any_final_get_no_synthesized_cursor() {
        // The per-line renderer always emits a `$0`; when fed bodies without
        // one, the merge does not invent it (an LSP snippet without `$0`
        // ends at its end implicitly).
        assert_eq!(merge(&[("", "$1"), ("", "$1")]), "$1\n$2\n");
    }
}
