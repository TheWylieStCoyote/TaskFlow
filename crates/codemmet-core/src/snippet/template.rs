//! Parser for TOML snippet `body` strings into the [`Template`] IR.
//!
//! Body syntax is LSP snippet syntax (`$1`, `${1:placeholder}`, `$0`, with
//! `\` escaping `$`, `}`, `\`) plus two extensions: `${child}` (composition
//! target, at most one) and `${name}` (named parameter reference).
//!
//! Indentation rules: each leading tab is one depth level; runs of exactly
//! four spaces are accepted and normalized to one level; mixing tabs and
//! spaces in one line's indent, or space runs not divisible by four, are
//! errors.

use super::ir::{IrLine, Span, Template};

/// A template body that violates the schema rules.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct TemplateError(pub String);

/// Per-line character stream with one-character lookahead.
type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

/// Parse a snippet `body` string into the line/span IR.
pub fn parse_template(body: &str) -> Result<Template, TemplateError> {
    let mut raw_lines: Vec<&str> = body.split('\n').collect();
    // A trailing newline (the usual TOML `'''` artifact) yields one final
    // empty element; drop it. Interior blank lines are preserved.
    if raw_lines.last() == Some(&"") {
        raw_lines.pop();
    }

    let mut lines = Vec::with_capacity(raw_lines.len());
    let mut saw_child = false;
    for (index, raw) in raw_lines.iter().enumerate() {
        let line_no = index + 1;
        let (depth, content) = parse_indent(raw, line_no)?;
        let spans = parse_spans(content, line_no, &mut saw_child)?;
        lines.push(IrLine { depth, spans });
    }
    Ok(Template { lines })
}

/// Build a [`TemplateError`] tagged with a 1-based line number.
fn err(line_no: usize, message: &str) -> TemplateError {
    TemplateError(format!("line {line_no}: {message}"))
}

/// Split a raw line into its indent depth and remaining content.
fn parse_indent(line: &str, line_no: usize) -> Result<(u32, &str), TemplateError> {
    let content = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - content.len()];
    let tabs = indent.bytes().filter(|&b| b == b'\t').count();
    let spaces = indent.len() - tabs;

    let depth = match (tabs, spaces) {
        (0, 0) => 0,
        (t, 0) => t,
        (0, s) if s % 4 == 0 => s / 4,
        (0, _) => {
            return Err(err(
                line_no,
                "indentation must be tabs or groups of 4 spaces",
            ));
        }
        (_, _) => return Err(err(line_no, "mixed tabs and spaces in indentation")),
    };
    let depth = u32::try_from(depth).map_err(|_| err(line_no, "indentation is too deep"))?;
    Ok((depth, content))
}

/// Parse a line's post-indent content into spans.
///
/// Adjacent literal runs coalesce into one [`Span::Text`]; empty text spans
/// are never emitted.
fn parse_spans(
    content: &str,
    line_no: usize,
    saw_child: &mut bool,
) -> Result<Vec<Span>, TemplateError> {
    let mut spans = Vec::new();
    let mut text = String::new();
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.peek().copied() {
                Some(escaped @ ('$' | '}' | '\\')) => {
                    chars.next();
                    text.push(escaped);
                }
                // A backslash before anything else is a literal backslash.
                _ => text.push('\\'),
            },
            '$' => match chars.peek().copied() {
                Some('{') => {
                    chars.next();
                    flush_text(&mut text, &mut spans);
                    spans.push(parse_braced(&mut chars, line_no, saw_child)?);
                }
                Some(d) if d.is_ascii_digit() => {
                    let index = parse_index(&mut chars, line_no)?;
                    flush_text(&mut text, &mut spans);
                    spans.push(numbered_span(index, None));
                }
                Some(a) if a.is_ascii_alphabetic() || a == '_' => {
                    return Err(err(line_no, "use ${name} for parameters"));
                }
                // A lone `$` is literal text.
                _ => text.push('$'),
            },
            _ => text.push(c),
        }
    }
    flush_text(&mut text, &mut spans);
    Ok(spans)
}

/// Move any buffered literal text into `spans` as a [`Span::Text`].
fn flush_text(text: &mut String, spans: &mut Vec<Span>) {
    if !text.is_empty() {
        spans.push(Span::Text(std::mem::take(text)));
    }
}

/// Build the span for a numbered form: `$0`/`${0}` is the final cursor.
fn numbered_span(index: u32, placeholder: Option<String>) -> Span {
    if index == 0 {
        Span::FinalCursor
    } else {
        Span::Tabstop { index, placeholder }
    }
}

/// Parse the inside of a `${...}` expression; the `${` is already consumed.
fn parse_braced(
    chars: &mut Chars<'_>,
    line_no: usize,
    saw_child: &mut bool,
) -> Result<Span, TemplateError> {
    match chars.peek().copied() {
        Some(c) if c.is_ascii_digit() => {
            let index = parse_index(chars, line_no)?;
            match chars.next() {
                Some('}') => Ok(numbered_span(index, None)),
                Some(':') => {
                    if index == 0 {
                        return Err(err(line_no, "the final cursor cannot have a placeholder"));
                    }
                    let placeholder = parse_placeholder(chars, line_no)?;
                    Ok(Span::Tabstop {
                        index,
                        placeholder: Some(placeholder),
                    })
                }
                _ => Err(err(line_no, "expected '}' or ':' after tabstop index")),
            }
        }
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
            let mut name = String::new();
            while let Some(c) = chars.peek().copied() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    chars.next();
                    name.push(c);
                } else {
                    break;
                }
            }
            // `${value}` / `${value.key}` / `${value:default}`: the data-driven
            // repeat item. Handled before the `}` check because of the `.`/`:`.
            if name == "value" {
                let member = if chars.peek() == Some(&'.') {
                    chars.next();
                    let mut key = String::new();
                    while let Some(c) = chars.peek().copied() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            chars.next();
                            key.push(c);
                        } else {
                            break;
                        }
                    }
                    if key.is_empty() {
                        return Err(err(line_no, "expected a member name after ${value."));
                    }
                    Some(key)
                } else {
                    None
                };
                return match chars.next() {
                    Some('}') => Ok(Span::ItemValue {
                        member,
                        default: None,
                    }),
                    Some(':') => Ok(Span::ItemValue {
                        member,
                        default: Some(parse_placeholder(chars, line_no)?),
                    }),
                    _ => Err(err(line_no, "expected '}' or ':' in ${value}")),
                };
            }
            if chars.next() != Some('}') {
                return Err(err(line_no, "expected '}' after parameter name"));
            }
            match name.as_str() {
                "child" => {
                    if *saw_child {
                        return Err(err(line_no, "at most one ${child} per snippet"));
                    }
                    *saw_child = true;
                    Ok(Span::ChildSlot)
                }
                // `${i}` is the reserved repetition-index variable.
                "i" => Ok(Span::RepeatIndex),
                _ => Ok(Span::Param { name }),
            }
        }
        Some('}') => Err(err(line_no, "empty '${}'")),
        Some(_) => Err(err(line_no, "invalid '${...}' expression")),
        None => Err(err(line_no, "unclosed '${'")),
    }
}

/// Parse a run of ASCII digits into a tabstop index.
fn parse_index(chars: &mut Chars<'_>, line_no: usize) -> Result<u32, TemplateError> {
    let mut value: u32 = 0;
    while let Some(digit) = chars.peek().and_then(|c| c.to_digit(10)) {
        chars.next();
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(digit))
            .ok_or_else(|| err(line_no, "tabstop index is too large"))?;
    }
    Ok(value)
}

/// Parse placeholder text up to the closing `}`; the leading `:` is consumed.
///
/// Placeholder text is literal apart from `\}`, `\$`, `\\` escapes; nested
/// tabstops or placeholders are rejected.
fn parse_placeholder(chars: &mut Chars<'_>, line_no: usize) -> Result<String, TemplateError> {
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '}' => return Ok(out),
            '\\' => match chars.peek().copied() {
                Some(escaped @ ('$' | '}' | '\\')) => {
                    chars.next();
                    out.push(escaped);
                }
                _ => out.push('\\'),
            },
            '$' => match chars.peek().copied() {
                Some(d) if d == '{' || d.is_ascii_digit() => {
                    return Err(err(line_no, "nested placeholders are not supported"));
                }
                _ => out.push('$'),
            },
            _ => out.push(c),
        }
    }
    Err(err(line_no, "unclosed '${'"))
}

#[cfg(test)]
// Snippet bodies like `${0:end}` are indistinguishable from format args to
// this lint; they are fixture data, not formatting mistakes.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::*;

    fn parse(body: &str) -> Template {
        parse_template(body).expect("template should parse")
    }

    fn parse_err(body: &str) -> String {
        parse_template(body)
            .expect_err("template should be rejected")
            .0
    }

    fn text(s: &str) -> Span {
        Span::Text(s.to_owned())
    }

    fn stop(index: u32) -> Span {
        Span::Tabstop {
            index,
            placeholder: None,
        }
    }

    #[test]
    fn simple_text() {
        let t = parse("hello world");
        assert_eq!(
            t.lines,
            vec![IrLine {
                depth: 0,
                spans: vec![text("hello world")]
            }]
        );
    }

    #[test]
    fn tabs_and_four_space_indent_are_equivalent() {
        assert_eq!(parse("a\n\tb"), parse("a\n    b"));
        assert_eq!(parse("a\n\tb").lines[1].depth, 1);
    }

    #[test]
    fn multi_level_depth() {
        let t = parse("a\n\tb\n\t\tc\n        d");
        let depths: Vec<u32> = t.lines.iter().map(|l| l.depth).collect();
        assert_eq!(depths, vec![0, 1, 2, 2]);
    }

    #[test]
    fn bare_tabstop() {
        assert_eq!(parse("$1$2").lines[0].spans, vec![stop(1), stop(2)]);
    }

    #[test]
    fn tabstop_with_placeholder() {
        assert_eq!(
            parse("${2:foo bar}").lines[0].spans,
            vec![Span::Tabstop {
                index: 2,
                placeholder: Some("foo bar".to_owned())
            }]
        );
    }

    #[test]
    fn final_cursor_bare_and_braced() {
        assert_eq!(parse("$0").lines[0].spans, vec![Span::FinalCursor]);
        assert_eq!(parse("${0}").lines[0].spans, vec![Span::FinalCursor]);
    }

    #[test]
    fn braced_number_without_placeholder() {
        assert_eq!(parse("${3}").lines[0].spans, vec![stop(3)]);
    }

    #[test]
    fn child_slot_line() {
        assert_eq!(
            parse("\t${child}").lines[0],
            IrLine {
                depth: 1,
                spans: vec![Span::ChildSlot]
            }
        );
    }

    #[test]
    fn child_slot_mixed_with_text_is_permitted() {
        assert_eq!(
            parse("do(${child});").lines[0].spans,
            vec![text("do("), Span::ChildSlot, text(");")]
        );
    }

    #[test]
    fn repeat_index_variable() {
        // `${i}` is the reserved repetition-index variable, not a parameter.
        assert_eq!(
            parse("field${i}: x").lines[0].spans,
            vec![text("field"), Span::RepeatIndex, text(": x")]
        );
    }

    #[test]
    fn params() {
        assert_eq!(
            parse("for ${var} in ${end}").lines[0].spans,
            vec![
                text("for "),
                Span::Param {
                    name: "var".to_owned()
                },
                text(" in "),
                Span::Param {
                    name: "end".to_owned()
                },
            ]
        );
    }

    #[test]
    fn repeated_names_and_indices_just_repeat() {
        assert_eq!(
            parse("${x} $1 ${x} $1").lines[0].spans,
            vec![
                Span::Param {
                    name: "x".to_owned()
                },
                text(" "),
                stop(1),
                text(" "),
                Span::Param {
                    name: "x".to_owned()
                },
                text(" "),
                stop(1),
            ]
        );
    }

    #[test]
    fn escapes_unescape_and_coalesce() {
        assert_eq!(
            parse(r"a\$1b\}c\\d").lines[0].spans,
            vec![text(r"a$1b}c\d")]
        );
    }

    #[test]
    fn backslash_before_other_char_is_literal() {
        assert_eq!(parse(r"a\nb\").lines[0].spans, vec![text(r"a\nb\")]);
    }

    #[test]
    fn lone_dollar_is_literal() {
        assert_eq!(parse("cost: $ 5$").lines[0].spans, vec![text("cost: $ 5$")]);
    }

    #[test]
    fn placeholder_escapes() {
        assert_eq!(
            parse(r"${1:a\}b\$c\\d}").lines[0].spans,
            vec![Span::Tabstop {
                index: 1,
                placeholder: Some(r"a}b$c\d".to_owned())
            }]
        );
    }

    #[test]
    fn interior_blank_lines_are_preserved() {
        let t = parse("a\n\nb");
        assert_eq!(
            t.lines[1],
            IrLine {
                depth: 0,
                spans: vec![]
            }
        );
        assert_eq!(t.lines.len(), 3);
    }

    #[test]
    fn single_trailing_newline_is_dropped() {
        assert_eq!(parse("a\n").lines.len(), 1);
        // Only one trailing empty line is an artifact; a doubled newline
        // keeps its interior blank line.
        assert_eq!(parse("a\n\n").lines.len(), 2);
    }

    #[test]
    fn plan_fixture_rust_fori() {
        let t = parse("for ${var} in 0..${end} {\n\t${child}\n}\n");
        assert_eq!(
            t.lines,
            vec![
                IrLine {
                    depth: 0,
                    spans: vec![
                        text("for "),
                        Span::Param {
                            name: "var".to_owned()
                        },
                        text(" in 0.."),
                        Span::Param {
                            name: "end".to_owned()
                        },
                        text(" {"),
                    ],
                },
                IrLine {
                    depth: 1,
                    spans: vec![Span::ChildSlot]
                },
                IrLine {
                    depth: 0,
                    spans: vec![text("}")]
                },
            ]
        );
    }

    #[test]
    fn plan_fixture_rust_fn_variant() {
        let t = parse("fn ${name}(&self$1) {\n\t${child}\n}\n");
        assert_eq!(
            t.lines[0].spans,
            vec![
                text("fn "),
                Span::Param {
                    name: "name".to_owned()
                },
                text("(&self"),
                stop(1),
                text(") {"),
            ]
        );
    }

    #[test]
    fn error_mixed_indent() {
        assert_eq!(
            parse_err("a\n\t b"),
            "line 2: mixed tabs and spaces in indentation"
        );
        assert_eq!(
            parse_err(" \tb"),
            "line 1: mixed tabs and spaces in indentation"
        );
    }

    #[test]
    fn error_bad_space_count() {
        assert_eq!(
            parse_err("   b"),
            "line 1: indentation must be tabs or groups of 4 spaces"
        );
        assert_eq!(
            parse_err("a\n      b"),
            "line 2: indentation must be tabs or groups of 4 spaces"
        );
    }

    #[test]
    fn error_two_child_slots() {
        assert_eq!(
            parse_err("${child}\n${child}"),
            "line 2: at most one ${child} per snippet"
        );
    }

    #[test]
    fn error_final_cursor_with_placeholder() {
        assert_eq!(
            parse_err("${0:end}"),
            "line 1: the final cursor cannot have a placeholder"
        );
    }

    #[test]
    fn error_nested_placeholder() {
        assert_eq!(
            parse_err("${1:${2:x}}"),
            "line 1: nested placeholders are not supported"
        );
        assert_eq!(
            parse_err("${1:a $2 b}"),
            "line 1: nested placeholders are not supported"
        );
    }

    #[test]
    fn error_bare_dollar_identifier() {
        assert_eq!(
            parse_err("echo $name"),
            "line 1: use ${name} for parameters"
        );
    }

    #[test]
    fn error_unclosed_brace_forms() {
        assert_eq!(parse_err("${"), "line 1: unclosed '${'");
        assert_eq!(parse_err("${1:foo"), "line 1: unclosed '${'");
        assert_eq!(
            parse_err("${1"),
            "line 1: expected '}' or ':' after tabstop index"
        );
        assert_eq!(
            parse_err("${name "),
            "line 1: expected '}' after parameter name"
        );
        assert_eq!(parse_err("${}"), "line 1: empty '${}'");
        assert_eq!(parse_err("${-}"), "line 1: invalid '${...}' expression");
    }

    #[test]
    fn empty_body_is_empty_template() {
        assert_eq!(parse("").lines.len(), 0);
    }
}
