//! Extracting the abbreviation candidate to the left of the cursor.
//!
//! Rather than scanning left over an abbreviation charset (which cannot get
//! quoting right when read backwards), we take the current line up to the
//! cursor and split it at the last whitespace that is *not* inside a quoted
//! region, tracking quote state left to right via [`QuoteScanner`].
//!
//! Quote state is grammar-aware: the abbreviation grammar only allows quoted
//! text as arguments, and an argument can only start right after `:` or `,`.
//! A quote anywhere else is a literal character — source lines are full of
//! them (Rust lifetimes, contractions in comments, string-literal bodies) and
//! treating each as an opener would swallow the rest of the line.

use codemmet_core::abbr::OPERATOR_CHARS;

/// Longest candidate accepted, in bytes.
const MAX_CANDIDATE_BYTES: usize = 200;

/// Classification of one scanned character. See [`QuoteScanner::step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// A backslash starting an escape.
    EscapeStart,
    /// A character consumed by a preceding backslash.
    Escaped,
    /// A quote opening a quoted argument region.
    Open,
    /// The quote closing the open region.
    Close,
    /// A character inside an open quoted region.
    Quoted,
    /// A plain unquoted, unescaped character (including literal quotes).
    Plain,
}

/// Shared left-to-right quote/escape state for the candidate scanners.
///
/// A quote character only *opens* a quoted region when it immediately
/// follows an argument introducer (`:` or `,`) — the only position the
/// abbreviation grammar allows quoted args. A quote elsewhere is a literal
/// character, though the matching quote of a legitimately opened region
/// still terminates it.
#[derive(Default)]
struct QuoteScanner {
    /// The opening quote of the active region, if any.
    quote: Option<char>,
    /// Whether the previous character was an unconsumed backslash.
    escaped: bool,
    /// Whether the previous character was an argument introducer.
    after_introducer: bool,
}

impl QuoteScanner {
    /// Advance over `c`, returning its classification.
    const fn step(&mut self, c: char) -> Step {
        if self.escaped {
            self.escaped = false;
            self.after_introducer = false;
            return Step::Escaped;
        }
        if c == '\\' {
            self.escaped = true;
            return Step::EscapeStart;
        }
        match self.quote {
            Some(q) if c == q => {
                self.quote = None;
                Step::Close
            }
            Some(_) => Step::Quoted,
            None if (c == '"' || c == '\'') && self.after_introducer => {
                self.quote = Some(c);
                self.after_introducer = false;
                Step::Open
            }
            None => {
                self.after_introducer = c == ':' || c == ',';
                Step::Plain
            }
        }
    }

    /// Whether the scan ended inside a quoted region or a dangling escape.
    const fn is_open(&self) -> bool {
        self.quote.is_some() || self.escaped
    }
}

/// The abbreviation candidate ending at byte `offset`, with its start offset.
///
/// Returns `None` when the candidate is empty or implausibly long.
pub fn candidate_at(text: &str, offset: usize) -> Option<(usize, &str)> {
    let offset = offset.min(text.len());
    let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line = &text[line_start..offset];
    let split = split_point(line);
    let candidate = &line[split..];
    if candidate.is_empty() || candidate.len() > MAX_CANDIDATE_BYTES {
        return None;
    }
    Some((line_start + split, candidate))
}

/// Byte offset just after the last unquoted, unescaped whitespace in `line`.
fn split_point(line: &str) -> usize {
    let mut scanner = QuoteScanner::default();
    let mut split = 0;
    for (i, c) in line.char_indices() {
        if scanner.step(c) == Step::Plain && c.is_whitespace() {
            split = i + c.len_utf8();
        }
    }
    split
}

/// Whether the candidate contains a composition operator outside quotes
/// (the gate for `completion = "operators-only"`).
pub fn has_operator(candidate: &str) -> bool {
    let mut scanner = QuoteScanner::default();
    candidate
        .chars()
        .any(|c| scanner.step(c) == Step::Plain && OPERATOR_CHARS.contains(&c))
}

/// The final segment of a candidate: the text after the last operator
/// character outside quotes, for trigger-prefix discovery.
///
/// Returns `None` when the cursor sits inside a quoted or escaped region or
/// the segment is empty or not a plausible trigger prefix (word charset).
pub fn final_segment(candidate: &str) -> Option<&str> {
    let mut scanner = QuoteScanner::default();
    let mut start = 0;
    for (i, c) in candidate.char_indices() {
        match scanner.step(c) {
            Step::Escaped | Step::Close => start = i + c.len_utf8(),
            Step::Plain if OPERATOR_CHARS.contains(&c) => start = i + c.len_utf8(),
            _ => {}
        }
    }
    if scanner.is_open() {
        return None;
    }
    let segment = &candidate[start..];
    let is_word = !segment.is_empty()
        && segment
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_');
    is_word.then_some(segment)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Candidate for a one-line document with the cursor at its end.
    fn candidate(line: &str) -> Option<(usize, &str)> {
        candidate_at(line, line.len())
    }

    #[test]
    fn bare_word_at_line_start() {
        assert_eq!(candidate("fori10"), Some((0, "fori10")));
    }

    #[test]
    fn candidate_starts_after_last_whitespace() {
        assert_eq!(candidate("    fori10"), Some((4, "fori10")));
        assert_eq!(candidate("let x = fn:main>pr"), Some((8, "fn:main>pr")));
    }

    #[test]
    fn quoted_whitespace_does_not_split() {
        assert_eq!(
            candidate("  pr:\"hello world\">ret"),
            Some((2, "pr:\"hello world\">ret"))
        );
        // An unterminated arg quote swallows the rest of the line.
        assert_eq!(candidate("x pr:\"a b"), Some((2, "pr:\"a b")));
        assert_eq!(candidate("x pr:'a b"), Some((2, "pr:'a b")));
        // A quoted arg keeps its comma and space out of splitting/segmenting.
        assert_eq!(candidate("td:\"a, b\">pr"), Some((0, "td:\"a, b\">pr")));
    }

    #[test]
    fn literal_quotes_do_not_open_regions() {
        // A Rust lifetime's apostrophe is a literal, not a quote opener.
        assert_eq!(candidate("let x: &'a str = y; pr"), Some((20, "pr")));
        // Quotes of a closed string literal earlier on the line.
        assert_eq!(candidate("let s = \"x y\"; pr"), Some((15, "pr")));
        // An apostrophe in prose (e.g. inside a comment).
        assert_eq!(candidate("it's pr"), Some((5, "pr")));
    }

    #[test]
    fn escaped_whitespace_does_not_split() {
        assert_eq!(candidate("x pr:a\\ b"), Some((2, "pr:a\\ b")));
    }

    #[test]
    fn whitespace_inside_one_quote_kind_but_not_the_other() {
        assert_eq!(
            candidate("pr:\"it's x\">ret"),
            Some((0, "pr:\"it's x\">ret"))
        );
    }

    #[test]
    fn empty_and_whitespace_only_lines_yield_none() {
        assert_eq!(candidate(""), None);
        assert_eq!(candidate("    "), None);
    }

    #[test]
    fn candidate_only_reaches_back_to_the_cursor_line() {
        let text = "fori10\npr";
        assert_eq!(candidate_at(text, text.len()), Some((7, "pr")));
    }

    #[test]
    fn candidate_respects_cursor_mid_line() {
        let text = "  fori10>pr";
        assert_eq!(candidate_at(text, 8), Some((2, "fori10")));
    }

    #[test]
    fn overlong_candidate_is_rejected() {
        let line = "a".repeat(201);
        assert_eq!(candidate(&line), None);
    }

    #[test]
    fn operator_gate() {
        assert!(!has_operator("fori10"));
        assert!(has_operator("fn:main"));
        assert!(has_operator("a>b"));
        assert!(has_operator("arm*3"));
        // Every grammar operator gates, not just a subset.
        assert!(has_operator("pr^ret"));
        assert!(has_operator("(if+el"));
    }

    #[test]
    fn caret_inside_quoted_arg_is_not_an_operator() {
        // `:` is itself an operator, so `has_operator` cannot isolate a
        // quoted `^`; assert on the shared scanner that the `^` (and the
        // rest of the arg) stays inside the quoted region.
        let mut scanner = QuoteScanner::default();
        for c in "pr:'a^b'".chars() {
            let step = scanner.step(c);
            if c == '^' {
                assert_eq!(step, Step::Quoted);
            }
        }
        assert!(!scanner.is_open());
    }

    #[test]
    fn final_segment_after_operators() {
        assert_eq!(final_segment("fo"), Some("fo"));
        assert_eq!(final_segment("fn:main>pr"), Some("pr"));
        assert_eq!(final_segment("fori10>"), None);
        assert_eq!(final_segment("(if+el"), Some("el"));
    }

    #[test]
    fn final_segment_suppressed_inside_quotes_and_escapes() {
        assert_eq!(final_segment("pr:\"hel"), None);
        assert_eq!(final_segment("pr:a\\"), None);
        assert_eq!(final_segment("pr:\"a b\""), None);
    }
}
