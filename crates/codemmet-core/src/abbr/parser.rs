//! Recursive-descent parser for the abbreviation mini-grammar.
//!
//! The lexer is folded into the parser as a byte-offset scanner rather than
//! a separate token stream: argument lexing is context-sensitive (`:` is
//! literal inside an argument, quotes and backslash escapes only matter
//! there), so a standalone lexer would need parser feedback anyway.
//!
//! Design notes:
//!
//! - `*n` on a plain node sets [`AbbrNode::repeat`]; the renderer expands it.
//! - `*n` on a parenthesised group is distributed at parse time into `n`
//!   sibling copies, because [`AbbrNode`] has no group concept. A `> group`
//!   following the element is attached before distribution, so each copy
//!   receives the children (matching Emmet's `li*3>a` semantics).
//! - `^` (climb-up) is reserved and rejected with a [`ParseError`] until it
//!   is implemented in M4.

use std::ops::Range;

use super::{AbbrNode, Abbreviation, ParseError};

/// Maximum accepted `*n` repeat count.
const MAX_REPEAT: u32 = 100;

/// Parse an abbreviation like `fn:main>fori10>pr` into its AST.
pub fn parse(input: &str) -> Result<Abbreviation, ParseError> {
    if input.is_empty() {
        return Err(err("empty abbreviation", 0..0));
    }
    let mut parser = Parser { input, pos: 0 };
    let nodes = parser.group()?;
    if parser.pos < input.len() {
        return Err(parser.unexpected_here());
    }
    Ok(Abbreviation(nodes))
}

/// Shorthand [`ParseError`] constructor.
fn err(message: impl Into<String>, span: Range<usize>) -> ParseError {
    ParseError {
        message: message.into(),
        span,
    }
}

/// True for characters that terminate a raw (unquoted) argument.
const fn ends_raw_arg(c: char) -> bool {
    matches!(c, '>' | '+' | '*' | ',' | '(' | ')' | '^')
}

/// The two shapes an `atom` production can take.
enum Atom {
    /// A bare `WORD args?` node.
    Node(AbbrNode),
    /// A parenthesised `( group )`, kept flat for distribution.
    Group(Vec<AbbrNode>),
}

/// Scanner state: the input text and the current byte offset.
struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Consume and return the next character, advancing by its UTF-8 width.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Consume `c` if it is next; report whether it was.
    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    /// Span of the character at the current position (empty at end of input).
    fn here(&self) -> Range<usize> {
        let end = self.peek().map_or(self.pos, |c| self.pos + c.len_utf8());
        self.pos..end
    }

    /// Error for a character the grammar cannot continue with.
    fn unexpected_here(&self) -> ParseError {
        let span = self.here();
        match self.peek() {
            None => err("unexpected end of input", span),
            Some('^') => err("'^' is reserved for climb-up and not implemented yet", span),
            Some(c) if c.is_whitespace() => err("unexpected whitespace", span),
            Some(c) => err(format!("unexpected '{c}'"), span),
        }
    }

    /// `group := chain ( '+' chain )*`
    fn group(&mut self) -> Result<Vec<AbbrNode>, ParseError> {
        let mut nodes = self.chain()?;
        while self.peek() == Some('+') {
            let op = self.here();
            self.bump();
            if self.peek().is_none() {
                return Err(err("dangling '+'", op));
            }
            nodes.extend(self.chain()?);
        }
        Ok(nodes)
    }

    /// `chain := element ( '>' group )?` where `element := atom ( '*' NUMBER )?`
    fn chain(&mut self) -> Result<Vec<AbbrNode>, ParseError> {
        let atom = self.atom()?;
        let repeat = if self.peek() == Some('*') {
            let star = self.here();
            self.bump();
            self.repeat_count(star)?
        } else {
            1
        };
        let children = if self.peek() == Some('>') {
            let op = self.here();
            self.bump();
            if self.peek().is_none() {
                return Err(err("dangling '>'", op));
            }
            self.group()?
        } else {
            Vec::new()
        };
        Ok(match atom {
            Atom::Node(mut node) => {
                node.repeat = repeat;
                node.children = children;
                vec![node]
            }
            Atom::Group(mut nodes) => {
                // `>` after a group nests into the group's last top-level
                // node (consistent with `a+b>c` nesting into `b`); the group
                // is never empty, so `last_mut` always matches.
                if let Some(last) = nodes.last_mut() {
                    last.children.extend(children);
                }
                // Distribute `(…)*n` into n sibling copies.
                if repeat > 1 {
                    let copy = nodes.clone();
                    for _ in 1..repeat {
                        nodes.extend_from_slice(&copy);
                    }
                }
                nodes
            }
        })
    }

    /// `atom := node | '(' group ')'`
    fn atom(&mut self) -> Result<Atom, ParseError> {
        match self.peek() {
            Some('(') => {
                let open = self.here();
                self.bump();
                let nodes = self.group()?;
                if self.eat(')') {
                    Ok(Atom::Group(nodes))
                } else if self.peek().is_none() {
                    Err(err("unclosed '('", open))
                } else {
                    Err(self.unexpected_here())
                }
            }
            Some(c) if c.is_ascii_alphabetic() || c == '_' => Ok(Atom::Node(self.node()?)),
            _ => Err(self.unexpected_here()),
        }
    }

    /// `node := WORD args?` with `args := ':' arg ( ',' arg )*`
    fn node(&mut self) -> Result<AbbrNode, ParseError> {
        let word = self.word();
        let mut args = Vec::new();
        if self.eat(':') {
            args.push(self.arg()?);
            while self.eat(',') {
                args.push(self.arg()?);
            }
        }
        Ok(AbbrNode {
            word,
            args,
            repeat: 1,
            children: Vec::new(),
        })
    }

    /// `WORD := [A-Za-z_][A-Za-z0-9_]*` — the caller has checked the first
    /// character, so the result is never empty.
    fn word(&mut self) -> String {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.bump();
        }
        self.input[start..self.pos].to_string()
    }

    /// The NUMBER after `*`; `star` is the operator's span for errors.
    fn repeat_count(&mut self, star: Range<usize>) -> Result<u32, ParseError> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
        if self.pos == start {
            return Err(err("expected a number after '*'", star));
        }
        let Ok(n) = self.input[start..self.pos].parse::<u32>() else {
            return Err(err("repeat count too large", start..self.pos));
        };
        if n == 0 {
            return Err(err("repeat count must be at least 1", start..self.pos));
        }
        if n > MAX_REPEAT {
            return Err(err("repeat count too large", start..self.pos));
        }
        Ok(n)
    }

    /// One argument: quoted (`"…"` / `'…'`) or raw. Returns the unescaped text.
    fn arg(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some(q @ ('"' | '\'')) => self.quoted_arg(q),
            _ => self.raw_arg(),
        }
    }

    /// A quoted argument. Backslash escapes the quote character and the
    /// backslash itself; any other escape is kept verbatim.
    fn quoted_arg(&mut self, quote: char) -> Result<String, ParseError> {
        let open = self.here();
        self.bump();
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(err("unclosed quote", open)),
                Some('\\') => match self.bump() {
                    None => return Err(err("unclosed quote", open)),
                    Some(c) if c == quote || c == '\\' => out.push(c),
                    Some(c) => {
                        out.push('\\');
                        out.push(c);
                    }
                },
                Some(c) if c == quote => return Ok(out),
                Some(c) => out.push(c),
            }
        }
    }

    /// A raw argument: any run of characters excluding unescaped operators
    /// (`> + * , ( ) ^`) and whitespace; backslash escapes the next character.
    /// `:` is literal here (`t:std::io` is one argument).
    fn raw_arg(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        let mut out = String::new();
        loop {
            match self.peek() {
                Some('\\') => {
                    let esc = self.here();
                    self.bump();
                    match self.bump() {
                        None => return Err(err("dangling escape", esc)),
                        Some(c) => out.push(c),
                    }
                }
                None => break,
                Some(c) if ends_raw_arg(c) || c.is_whitespace() => break,
                Some(c) => {
                    out.push(c);
                    self.bump();
                }
            }
        }
        if self.pos == start {
            if self.peek().is_some_and(char::is_whitespace) {
                return Err(self.unexpected_here());
            }
            return Err(err("expected an argument", self.here()));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{AbbrNode, ParseError};
    use super::parse;

    /// A bare node with the given word and all other fields defaulted.
    fn word(w: &str) -> AbbrNode {
        AbbrNode {
            word: w.to_string(),
            args: Vec::new(),
            repeat: 1,
            children: Vec::new(),
        }
    }

    fn parsed(input: &str) -> Vec<AbbrNode> {
        parse(input).expect("input should parse").0
    }

    fn parse_err(input: &str) -> ParseError {
        parse(input).expect_err("input should fail to parse")
    }

    #[test]
    fn single_word_stays_glued() {
        // `fori10` is one WORD; trigger/glued-arg splitting happens later.
        assert_eq!(parsed("fori10"), vec![word("fori10")]);
    }

    #[test]
    fn child_operator() {
        assert_eq!(
            parsed("a>b"),
            vec![AbbrNode {
                children: vec![word("b")],
                ..word("a")
            }]
        );
    }

    #[test]
    fn sibling_operator() {
        assert_eq!(parsed("a+b"), vec![word("a"), word("b")]);
    }

    #[test]
    fn child_captures_following_group() {
        // `a>b+c`: b and c are BOTH children of a.
        assert_eq!(
            parsed("a>b+c"),
            vec![AbbrNode {
                children: vec![word("b"), word("c")],
                ..word("a")
            }]
        );
    }

    #[test]
    fn sibling_then_child() {
        // `a+b>c`: c is a child of b; a is b's sibling.
        assert_eq!(
            parsed("a+b>c"),
            vec![
                word("a"),
                AbbrNode {
                    children: vec![word("c")],
                    ..word("b")
                },
            ]
        );
    }

    #[test]
    fn nested_capture() {
        // `a>b+c>d`: children of a are b and c; d is a child of c.
        assert_eq!(
            parsed("a>b+c>d"),
            vec![AbbrNode {
                children: vec![
                    word("b"),
                    AbbrNode {
                        children: vec![word("d")],
                        ..word("c")
                    },
                ],
                ..word("a")
            }]
        );
    }

    #[test]
    fn grouping_breaks_capture() {
        // `(a>b)+c`: c is a sibling of a, not a child.
        assert_eq!(
            parsed("(a>b)+c"),
            vec![
                AbbrNode {
                    children: vec![word("b")],
                    ..word("a")
                },
                word("c"),
            ]
        );
    }

    #[test]
    fn repeat_on_node() {
        assert_eq!(
            parsed("a*3"),
            vec![AbbrNode {
                repeat: 3,
                ..word("a")
            }]
        );
    }

    #[test]
    fn repeat_binds_tighter_than_child() {
        // `a*3>b`: the repeated node keeps its children (renderer clones them).
        assert_eq!(
            parsed("a*3>b"),
            vec![AbbrNode {
                repeat: 3,
                children: vec![word("b")],
                ..word("a")
            }]
        );
    }

    #[test]
    fn repeat_on_group_distributes() {
        // `(a>b)*2`: two sibling copies of the a>b chain.
        let copy = AbbrNode {
            children: vec![word("b")],
            ..word("a")
        };
        assert_eq!(parsed("(a>b)*2"), vec![copy.clone(), copy]);
    }

    #[test]
    fn repeated_group_with_children_gives_each_copy_the_children() {
        // `(a>b)*2>c`: children attach before distribution, so every copy
        // nests c into its last top-level node.
        let copy = AbbrNode {
            children: vec![word("b"), word("c")],
            ..word("a")
        };
        assert_eq!(parsed("(a>b)*2>c"), vec![copy.clone(), copy]);
    }

    #[test]
    fn repeated_group_as_children() {
        assert_eq!(
            parsed("a>(b+c)*2"),
            vec![AbbrNode {
                children: vec![word("b"), word("c"), word("b"), word("c")],
                ..word("a")
            }]
        );
    }

    #[test]
    fn single_arg() {
        assert_eq!(
            parsed("fn:main"),
            vec![AbbrNode {
                args: vec!["main".to_string()],
                ..word("fn")
            }]
        );
    }

    #[test]
    fn multiple_args() {
        assert_eq!(
            parsed("f:a,b,c"),
            vec![AbbrNode {
                args: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                ..word("f")
            }]
        );
    }

    #[test]
    fn quoted_arg_keeps_comma_and_whitespace() {
        assert_eq!(
            parsed(r#"td:"a, b""#),
            vec![AbbrNode {
                args: vec!["a, b".to_string()],
                ..word("td")
            }]
        );
    }

    #[test]
    fn quoted_arg_unescapes_quote_and_backslash() {
        assert_eq!(
            parsed(r#"td:"a\"b\\c",'d\'e'"#),
            vec![AbbrNode {
                args: vec![r#"a"b\c"#.to_string(), "d'e".to_string()],
                ..word("td")
            }]
        );
    }

    #[test]
    fn raw_arg_escapes_operator() {
        assert_eq!(
            parsed(r"pr:a\>b"),
            vec![AbbrNode {
                args: vec!["a>b".to_string()],
                ..word("pr")
            }]
        );
    }

    #[test]
    fn raw_arg_keeps_literal_colons() {
        assert_eq!(
            parsed("t:std::io"),
            vec![AbbrNode {
                args: vec!["std::io".to_string()],
                ..word("t")
            }]
        );
    }

    #[test]
    fn args_combine_with_children_and_repeat() {
        assert_eq!(
            parsed("fn:main>fori10*2>pr:x"),
            vec![AbbrNode {
                args: vec!["main".to_string()],
                children: vec![AbbrNode {
                    repeat: 2,
                    children: vec![AbbrNode {
                        args: vec!["x".to_string()],
                        ..word("pr")
                    }],
                    ..word("fori10")
                }],
                ..word("fn")
            }]
        );
    }

    #[test]
    fn error_empty_input() {
        let e = parse_err("");
        assert_eq!(e.message, "empty abbreviation");
        assert_eq!(e.span, 0..0);
    }

    #[test]
    fn error_dangling_child_operator() {
        let e = parse_err("a>");
        assert_eq!(e.message, "dangling '>'");
        assert_eq!(e.span, 1..2);
    }

    #[test]
    fn error_dangling_sibling_operator() {
        let e = parse_err("a+");
        assert_eq!(e.message, "dangling '+'");
        assert_eq!(e.span, 1..2);
    }

    #[test]
    fn error_leading_operator() {
        let e = parse_err(">a");
        assert_eq!(e.message, "unexpected '>'");
        assert_eq!(e.span, 0..1);
    }

    #[test]
    fn error_unclosed_paren() {
        let e = parse_err("(a>b");
        assert_eq!(e.message, "unclosed '('");
        assert_eq!(e.span, 0..1);
    }

    #[test]
    fn error_unclosed_quote() {
        let e = parse_err(r#"f:"ab"#);
        assert_eq!(e.message, "unclosed quote");
        assert_eq!(e.span, 2..3);
    }

    #[test]
    fn error_star_without_number() {
        let e = parse_err("a*");
        assert_eq!(e.message, "expected a number after '*'");
        assert_eq!(e.span, 1..2);

        let e = parse_err("a*x");
        assert_eq!(e.message, "expected a number after '*'");
        assert_eq!(e.span, 1..2);
    }

    #[test]
    fn error_repeat_zero() {
        let e = parse_err("a*0");
        assert_eq!(e.message, "repeat count must be at least 1");
        assert_eq!(e.span, 2..3);
    }

    #[test]
    fn error_repeat_too_large() {
        let e = parse_err("a*101");
        assert_eq!(e.message, "repeat count too large");
        assert_eq!(e.span, 2..5);

        // Also past u32::MAX.
        let e = parse_err("a*99999999999999999999");
        assert_eq!(e.message, "repeat count too large");
    }

    #[test]
    fn error_caret_is_reserved() {
        let e = parse_err("a^b");
        assert!(e.message.contains("reserved"), "message: {}", e.message);
        assert_eq!(e.span, 1..2);
    }

    #[test]
    fn caret_is_literal_when_quoted_or_escaped() {
        assert_eq!(
            parsed(r#"a:"x^y",p\^q"#),
            vec![AbbrNode {
                args: vec!["x^y".to_string(), "p^q".to_string()],
                ..word("a")
            }]
        );
    }

    #[test]
    fn error_whitespace_outside_quotes() {
        let e = parse_err("a b");
        assert_eq!(e.message, "unexpected whitespace");
        assert_eq!(e.span, 1..2);

        let e = parse_err("f: x");
        assert_eq!(e.message, "unexpected whitespace");
        assert_eq!(e.span, 2..3);
    }

    #[test]
    fn error_unexpected_character() {
        let e = parse_err("a.b");
        assert_eq!(e.message, "unexpected '.'");
        assert_eq!(e.span, 1..2);
    }

    #[test]
    fn error_missing_argument() {
        let e = parse_err("f:");
        assert_eq!(e.message, "expected an argument");
        assert_eq!(e.span, 2..2);

        let e = parse_err("f:a,");
        assert_eq!(e.message, "expected an argument");
        assert_eq!(e.span, 4..4);
    }

    #[test]
    fn error_dangling_escape() {
        let e = parse_err(r"f:a\");
        assert_eq!(e.message, "dangling escape");
        assert_eq!(e.span, 3..4);
    }
}
