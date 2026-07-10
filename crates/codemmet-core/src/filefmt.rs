//! The `.codemmet` source-file format: line classification for file mode.
//!
//! A `.codemmet` file is UTF-8 text rendered line by line (see
//! `FILE_MODE_PLAN.md`): directives (`#lang rust`, `#repeat Msg=Quit,Move`)
//! configure expansion, `#`-space lines are comments, blank lines pass
//! through, and every other line is one abbreviation whose leading
//! whitespace is that line's base indent.
//!
//! This module is pure classification — no IO, no expansion, no state. The
//! CLI's file-mode driver owns the semantic rules (`#lang` placement,
//! collection precedence, output buffering); keeping the parser here makes
//! it unit-testable, fuzzable, and reusable outside the binary.

/// Directive names this version understands.
///
/// By design a directive mirrors the long CLI flag of the same name and
/// reuses its value grammar, so future flags slot in here without any
/// namespace work.
pub const DIRECTIVES: &[&str] = &["lang", "repeat"];

/// One classified line of a `.codemmet` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line<'a> {
    /// `#name value` — a known directive and its (possibly empty) value.
    Directive {
        /// The directive name, without the leading `#`.
        name: &'a str,
        /// The rest of the line, whitespace-trimmed. May be empty.
        value: &'a str,
    },
    /// `# freeform note` or a bare `#` — produces no output.
    Comment,
    /// A whitespace-only line — one blank line in the output.
    Blank,
    /// An abbreviation to expand.
    Abbrev {
        /// The line's leading whitespace, used as its base indent.
        indent: &'a str,
        /// The abbreviation text (trimmed on both sides).
        abbr: &'a str,
    },
}

/// A line that could not be classified.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LineError {
    /// `#word` where `word` is not a known directive. Typos must fail
    /// loudly rather than be silently dropped as comments.
    #[error("unknown directive `#{0}` (known: #lang, #repeat; start a comment with `# `)")]
    UnknownDirective(String),
}

/// Classify one raw line (without its trailing newline).
///
/// Trailing whitespace — including a CRLF `\r` — is ignored. A line whose
/// first non-whitespace character is `#` is a directive or comment, so
/// directives and comments may be indented; an abbreviation therefore can
/// never begin with `#`.
///
/// # Errors
/// [`LineError::UnknownDirective`] when `#` is followed by a word that is
/// not in [`DIRECTIVES`].
pub fn classify(raw: &str) -> Result<Line<'_>, LineError> {
    let line = raw.trim_end();
    let indent_len = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent_len);
    let Some(after_hash) = rest.strip_prefix('#') else {
        if rest.is_empty() {
            return Ok(Line::Blank);
        }
        return Ok(Line::Abbrev { indent, abbr: rest });
    };
    // `#` followed by whitespace or end of line is a comment; `#word` must
    // name a known directive.
    if after_hash.is_empty() || after_hash.starts_with(char::is_whitespace) {
        return Ok(Line::Comment);
    }
    let (name, value) = after_hash
        .split_once(char::is_whitespace)
        .unwrap_or((after_hash, ""));
    if !DIRECTIVES.contains(&name) {
        return Err(LineError::UnknownDirective(name.to_string()));
    }
    Ok(Line::Directive {
        name,
        value: value.trim(),
    })
}

/// Classify every line of a file, yielding 1-based line numbers.
///
/// Lines are split on `\n`; a trailing `\r` (CRLF input) is treated as
/// trailing whitespace by [`classify`].
pub fn lines(text: &str) -> impl Iterator<Item = (usize, Result<Line<'_>, LineError>)> {
    text.lines()
        .enumerate()
        .map(|(idx, raw)| (idx + 1, classify(raw)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_blank_comment_directive_and_abbrev() {
        assert_eq!(classify(""), Ok(Line::Blank));
        assert_eq!(classify("   \t"), Ok(Line::Blank));
        assert_eq!(classify("#"), Ok(Line::Comment));
        assert_eq!(classify("# a note"), Ok(Line::Comment));
        assert_eq!(
            classify("#lang rust"),
            Ok(Line::Directive {
                name: "lang",
                value: "rust"
            })
        );
        assert_eq!(
            classify("fn:main>pr"),
            Ok(Line::Abbrev {
                indent: "",
                abbr: "fn:main>pr"
            })
        );
    }

    #[test]
    fn abbrev_captures_leading_whitespace_as_indent() {
        assert_eq!(
            classify("    match:self>arm*@Msg"),
            Ok(Line::Abbrev {
                indent: "    ",
                abbr: "match:self>arm*@Msg"
            })
        );
        assert_eq!(
            classify("\tpr"),
            Ok(Line::Abbrev {
                indent: "\t",
                abbr: "pr"
            })
        );
    }

    #[test]
    fn directives_and_comments_may_be_indented() {
        assert_eq!(
            classify("  #repeat Msg=Quit,Move"),
            Ok(Line::Directive {
                name: "repeat",
                value: "Msg=Quit,Move"
            })
        );
        assert_eq!(classify("  # indented note"), Ok(Line::Comment));
    }

    #[test]
    fn unknown_directive_is_a_hard_error() {
        assert_eq!(
            classify("#repaet Msg=Quit"),
            Err(LineError::UnknownDirective("repaet".to_string()))
        );
        assert_eq!(
            classify("#Lang rust"),
            Err(LineError::UnknownDirective("Lang".to_string()))
        );
    }

    #[test]
    fn crlf_and_trailing_whitespace_are_ignored() {
        assert_eq!(
            classify("#lang rust\r"),
            Ok(Line::Directive {
                name: "lang",
                value: "rust"
            })
        );
        assert_eq!(classify("   \r"), Ok(Line::Blank));
        assert_eq!(
            classify("pr   \r"),
            Ok(Line::Abbrev {
                indent: "",
                abbr: "pr"
            })
        );
    }

    #[test]
    fn directive_value_may_be_empty_or_tab_separated() {
        assert_eq!(
            classify("#lang"),
            Ok(Line::Directive {
                name: "lang",
                value: ""
            })
        );
        assert_eq!(
            classify("#repeat\tE=A,B"),
            Ok(Line::Directive {
                name: "repeat",
                value: "E=A,B"
            })
        );
    }

    #[test]
    fn lines_yields_one_based_numbers() {
        let text = "#lang rust\n\npr\n";
        let numbered: Vec<usize> = lines(text).map(|(n, _)| n).collect();
        assert_eq!(numbered, [1, 2, 3]);
    }
}
