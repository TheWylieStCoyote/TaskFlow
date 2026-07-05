//! Extracting the abbreviation candidate to the left of the cursor.
//!
//! Rather than scanning left over an abbreviation charset (which cannot get
//! quoting right when read backwards), we take the current line up to the
//! cursor and split it at the last whitespace that is *not* inside a quoted
//! region, tracking quote state left to right. Everything after that split
//! point is the candidate.

/// Longest candidate accepted, in bytes.
const MAX_CANDIDATE_BYTES: usize = 200;

/// Abbreviation operator characters, for segment splitting and gating.
const OPERATORS: [char; 8] = ['>', '+', '*', ':', ',', '(', ')', '^'];

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
    let mut split = 0;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, c) {
            (_, '\\') => escaped = true,
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, c) if c.is_whitespace() => split = i + c.len_utf8(),
            _ => {}
        }
    }
    split
}

/// Whether the candidate contains a composition operator outside quotes
/// (the gate for `completion = "operators-only"`).
pub fn has_operator(candidate: &str) -> bool {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in candidate.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, c) {
            (_, '\\') => escaped = true,
            (Some(q), c) if c == q => quote = None,
            (None, '>' | '+' | '*' | ':') => return true,
            (None, '"' | '\'') => quote = Some(c),
            _ => {}
        }
    }
    false
}

/// The final segment of a candidate: the text after the last operator
/// character outside quotes, for trigger-prefix discovery.
///
/// Returns `None` when the cursor sits inside a quoted or escaped region or
/// the segment is empty or not a plausible trigger prefix (word charset).
pub fn final_segment(candidate: &str) -> Option<&str> {
    let mut start = 0;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in candidate.char_indices() {
        if escaped {
            escaped = false;
            start = i + c.len_utf8();
            continue;
        }
        match (quote, c) {
            (_, '\\') => escaped = true,
            (Some(q), c) if c == q => {
                quote = None;
                start = i + c.len_utf8();
            }
            (None, '"' | '\'') => quote = Some(c),
            (None, c) if OPERATORS.contains(&c) => start = i + c.len_utf8(),
            _ => {}
        }
    }
    if quote.is_some() || escaped {
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
        // An unterminated quote swallows the rest of the line.
        assert_eq!(candidate("x pr:\"a b"), Some((2, "pr:\"a b")));
        assert_eq!(candidate("x pr:'a b"), Some((2, "pr:'a b")));
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
        // Operators hidden inside quotes do not count.
        assert!(!has_operator("\"a>b\""));
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
