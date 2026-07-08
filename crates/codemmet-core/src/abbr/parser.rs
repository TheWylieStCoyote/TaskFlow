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
//! - `^` (climb-up) ends the current `>`-nesting level: the following chain
//!   attaches N levels up, where N is the number of consecutive carets.
//!   Rather than an explicit level stack, the climb count travels up the
//!   recursion: [`Parser::level`] returns its siblings plus a pending climb
//!   count; each enclosing `>` in [`Parser::chain_into`] consumes one level,
//!   and whichever level the count reaches zero at continues parsing there.
//!   [`Parser::top_group`] (the whole abbreviation, or one `(…)` group)
//!   clamps counts that would escape it, so extra carets stop at the top
//!   level and a `^` inside parentheses never climbs out of its group.

use std::ops::Range;

use super::{AbbrNode, Abbreviation, ParseError, Repeat};

/// Maximum accepted `*n` repeat count.
const MAX_REPEAT: u32 = 100;

/// Maximum weighted node count an abbreviation may materialize.
///
/// Group distribution clones subtrees at parse time, so nested `(…)*n`
/// multiplies: `((a*100)*100)*100` would materialize 10^4 nodes rendering
/// 10^6 copies. Nodes are charged by their render weight (repeat count,
/// times the subtree for distributed copies), so oversized inputs fail
/// during the parse instead of exhausting memory.
const MAX_NODES: usize = 10_000;

/// Parse an abbreviation like `fn:main>fori10>pr` into its AST.
pub fn parse(input: &str) -> Result<Abbreviation, ParseError> {
    if input.is_empty() {
        return Err(err("empty abbreviation", 0..0));
    }
    let mut parser = Parser {
        input,
        pos: 0,
        nodes: 0,
    };
    let nodes = parser.top_group()?;
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

/// How parsing continues after one chain within a level.
enum Flow {
    /// No climb: the level inspects the next character (`+`, `^`, `)`, end).
    Normal,
    /// A `^` climb ended a nested level. The count is how many levels
    /// *above the current one* the next chain belongs to; `Climb(0)` means
    /// it lands at the current level, which parses another chain directly.
    Climb(u32),
}

/// The two shapes an `atom` production can take.
enum Atom {
    /// A bare `WORD args?` node.
    Node(AbbrNode),
    /// A parenthesised `( group )`, kept flat for distribution.
    Group(Vec<AbbrNode>),
}

/// Render-copy weight of a node: its repeat count times itself plus its
/// subtree. This is what one distributed clone of the node costs.
fn node_weight(node: &AbbrNode) -> usize {
    let children = node
        .children
        .iter()
        .fold(0usize, |sum, c| sum.saturating_add(node_weight(c)));
    usize::try_from(node.repeat.budget_weight())
        .unwrap_or(usize::MAX)
        .saturating_mul(children.saturating_add(1))
}

/// Scanner state: the input text and the current byte offset.
struct Parser<'a> {
    input: &'a str,
    pos: usize,
    /// Weighted count of materialized nodes, capped at [`MAX_NODES`].
    nodes: usize,
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
            Some('^') => err("'^' climb-up must follow an element", span),
            Some(c) if c.is_whitespace() => err("unexpected whitespace", span),
            Some(c) => err(format!("unexpected '{c}'"), span),
        }
    }

    /// A whole climb scope: the full abbreviation or one `(…)` group.
    ///
    /// Climb counts that would escape the scope's top level clamp here:
    /// the remaining carets are absorbed and parsing continues at this
    /// level (`a^^^b` is `a+b`; a `^` inside parentheses never climbs out).
    fn top_group(&mut self) -> Result<Vec<AbbrNode>, ParseError> {
        let (mut nodes, mut climb) = self.level()?;
        while climb > 0 {
            let (more, rest) = self.level()?;
            nodes.extend(more);
            climb = rest;
        }
        Ok(nodes)
    }

    /// One nesting level: `chain ( '+' chain )*`, ended by a climb, `)`, or
    /// the end of input. Returns the siblings collected at this level plus
    /// the pending climb count: how many levels above this one the next
    /// chain belongs to (`0` when the level ended without a climb).
    fn level(&mut self) -> Result<(Vec<AbbrNode>, u32), ParseError> {
        let mut nodes = Vec::new();
        loop {
            match self.chain_into(&mut nodes)? {
                // A nested climb landed exactly here: parse the next chain.
                Flow::Climb(0) => continue,
                Flow::Climb(pending) => return Ok((nodes, pending)),
                Flow::Normal => {}
            }
            match self.peek() {
                Some('+') => {
                    let op = self.here();
                    self.bump();
                    if self.peek().is_none() {
                        return Err(err("dangling '+'", op));
                    }
                }
                Some('^') => return Ok((nodes, self.carets()?)),
                _ => return Ok((nodes, 0)),
            }
        }
    }

    /// A caret run `'^'+`: consume it and return the climb count.
    fn carets(&mut self) -> Result<u32, ParseError> {
        let start = self.pos;
        let mut count: u32 = 0;
        while self.eat('^') {
            count = count.saturating_add(1);
        }
        if self.peek().is_none() {
            return Err(err("dangling '^'", start..self.pos));
        }
        Ok(count)
    }

    /// Record `weight` materialized nodes; fail past [`MAX_NODES`].
    ///
    /// `span` points at the offending `*n` number (or the whole input when
    /// the overflow has no single repeat to blame).
    fn charge_nodes(&mut self, weight: usize, span: &Range<usize>) -> Result<(), ParseError> {
        self.nodes = self.nodes.saturating_add(weight);
        if self.nodes > MAX_NODES {
            return Err(err("abbreviation too large", span.clone()));
        }
        Ok(())
    }

    /// `chain := element ( '>' level )?` where `element := atom ( '*' NUMBER )?`.
    ///
    /// Appends the chain's node(s) to `nodes` and reports how parsing
    /// continues: [`Flow::Normal`], or [`Flow::Climb`] when the nested level
    /// ended with `^` carets left over after consuming one for this `>`.
    fn chain_into(&mut self, nodes: &mut Vec<AbbrNode>) -> Result<Flow, ParseError> {
        let atom = self.atom()?;
        let (repeat, repeat_span) = if self.peek() == Some('*') {
            let star = self.here();
            self.bump();
            let start = self.pos;
            let repeat = self.repeat_spec(star)?;
            (repeat, start..self.pos)
        } else {
            (Repeat::Count(1), 0..self.input.len())
        };
        let (children, flow) = if self.peek() == Some('>') {
            let op = self.here();
            self.bump();
            if self.peek().is_none() {
                return Err(err("dangling '>'", op));
            }
            let (children, climb) = self.level()?;
            let flow = match climb {
                0 => Flow::Normal,
                pending => Flow::Climb(pending - 1),
            };
            (children, flow)
        } else {
            (Vec::new(), Flow::Normal)
        };
        match atom {
            Atom::Node(mut node) => {
                self.charge_nodes(
                    usize::try_from(repeat.budget_weight()).unwrap_or(usize::MAX),
                    &repeat_span,
                )?;
                node.repeat = repeat;
                node.children = children;
                nodes.push(node);
            }
            Atom::Group(mut group) => {
                // `>` after a group nests into the group's last top-level
                // node (consistent with `a+b>c` nesting into `b`); the group
                // is never empty, so `last_mut` always matches.
                if let Some(last) = group.last_mut() {
                    last.children.extend(children);
                }
                // A data-driven count can't be distributed at parse time (it is
                // unknown until render); support it on plain nodes only.
                if repeat.is_data_driven() {
                    return Err(err(
                        "data-driven repeat (`*@name`, `*`, or `*[…]`) is not supported on a group",
                        repeat_span,
                    ));
                }
                // Distribute `(…)*n` into n sibling copies, charging each
                // copy's full subtree weight so nested distribution fails
                // fast instead of materializing the clones.
                let count = repeat.budget_weight();
                if count > 1 {
                    let weight = group
                        .iter()
                        .fold(0usize, |sum, n| sum.saturating_add(node_weight(n)));
                    let copy = group.clone();
                    for _ in 1..count {
                        self.charge_nodes(weight, &repeat_span)?;
                        group.extend_from_slice(&copy);
                    }
                }
                nodes.extend(group);
            }
        }
        Ok(flow)
    }

    /// `atom := node | '(' group ')'`
    fn atom(&mut self) -> Result<Atom, ParseError> {
        match self.peek() {
            Some('(') => {
                let open = self.here();
                self.bump();
                let nodes = self.top_group()?;
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
            repeat: Repeat::Count(1),
            children: Vec::new(),
        })
    }

    /// The repeat spec after a `*`: `@name` (data-driven source), a NUMBER
    /// (literal count), or nothing/an operator (a bare `*`, data-driven over
    /// the enclosing subject).
    fn repeat_spec(&mut self, star: Range<usize>) -> Result<Repeat, ParseError> {
        match self.peek() {
            Some('@') => {
                self.bump();
                if !self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                {
                    return Err(err("expected a name after '*@'", star));
                }
                Ok(Repeat::Source(self.word()))
            }
            Some('[') => self.inline_spec(star),
            Some(c) if c.is_ascii_digit() => Ok(Repeat::Count(self.repeat_count(star)?)),
            _ => Ok(Repeat::Subject),
        }
    }

    /// The inline collection literal after `*`: `[a,b,c]` (each value optionally
    /// `value:type`). The raw `[…]` body is captured and parsed at render time.
    fn inline_spec(&mut self, star: Range<usize>) -> Result<Repeat, ParseError> {
        self.bump(); // consume '['
        let start = self.pos;
        while self.peek().is_some_and(|c| c != ']') {
            self.bump();
        }
        if self.peek() != Some(']') {
            return Err(err(
                "unclosed inline collection: expected `]` (as in `*[a,b,c]`)",
                star,
            ));
        }
        let inner = self.input[start..self.pos].to_string();
        self.bump(); // consume ']'
        if inner.split(',').all(|v| v.trim().is_empty()) {
            return Err(err(
                "inline collection `*[…]` needs at least one value",
                start..self.pos,
            ));
        }
        Ok(Repeat::Inline(inner))
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
    use super::super::{AbbrNode, ParseError, Repeat};
    use super::parse;

    /// A bare node with the given word and all other fields defaulted.
    fn word(w: &str) -> AbbrNode {
        AbbrNode {
            word: w.to_string(),
            args: Vec::new(),
            repeat: Repeat::Count(1),
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
                repeat: Repeat::Count(3),
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
                repeat: Repeat::Count(3),
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
                    repeat: Repeat::Count(2),
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
    fn bare_star_is_data_driven_over_the_subject() {
        let nodes = parse("a*").expect("a bare `*` is valid (data-driven)").0;
        assert_eq!(nodes[0].repeat, Repeat::Subject);
    }

    #[test]
    fn star_at_names_an_explicit_source() {
        let nodes = parse("a*@Msg").expect("`*@name` is valid").0;
        assert_eq!(nodes[0].repeat, Repeat::Source("Msg".to_string()));
    }

    #[test]
    fn star_bracket_captures_an_inline_collection() {
        let nodes = parse("a*[new,len,min]").expect("`*[…]` is valid").0;
        assert_eq!(nodes[0].repeat, Repeat::Inline("new,len,min".to_string()));
        assert!(nodes[0].repeat.is_data_driven());
        assert_eq!(nodes[0].repeat.budget_weight(), 3);
        // Members and surrounding whitespace are preserved verbatim in the body.
        let nodes = parse("a*[ x:f64 , y:i32 ]").expect("members are valid").0;
        assert_eq!(
            nodes[0].repeat,
            Repeat::Inline(" x:f64 , y:i32 ".to_string())
        );
    }

    #[test]
    fn error_inline_collection_empty_or_unclosed() {
        assert!(parse("a*[]").is_err(), "empty `*[]` should be rejected");
        assert!(
            parse("a*[ , ]").is_err(),
            "all-empty items should be rejected"
        );
        assert!(
            parse("a*[new,len").is_err(),
            "unclosed `*[` should be rejected"
        );
    }

    #[test]
    fn error_star_at_without_a_name() {
        let e = parse_err("a*@");
        assert_eq!(e.message, "expected a name after '*@'");
    }

    #[test]
    fn error_data_driven_repeat_on_a_group() {
        let e = parse_err("(a+b)*@x");
        assert!(e.message.contains("not supported on a group"));
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
    fn climb_from_child_to_top() {
        // `a>b^c`: c climbs out of a's children back to the top level.
        assert_eq!(
            parsed("a>b^c"),
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
    fn climb_one_level_lands_midway() {
        // `a>b>c^d`: one caret from c's level makes d a sibling of b.
        assert_eq!(
            parsed("a>b>c^d"),
            vec![AbbrNode {
                children: vec![
                    AbbrNode {
                        children: vec![word("c")],
                        ..word("b")
                    },
                    word("d"),
                ],
                ..word("a")
            }]
        );
    }

    #[test]
    fn climb_two_levels() {
        // `a>b>c^^d`: two carets from c's level put d at the top level.
        assert_eq!(
            parsed("a>b>c^^d"),
            vec![
                AbbrNode {
                    children: vec![AbbrNode {
                        children: vec![word("c")],
                        ..word("b")
                    }],
                    ..word("a")
                },
                word("d"),
            ]
        );
    }

    #[test]
    fn climb_clamps_at_top_level() {
        // Extra carets past the top level clamp there (Emmet-compatible).
        assert_eq!(parsed("a^^^b"), vec![word("a"), word("b")]);
    }

    #[test]
    fn climb_then_descend_again() {
        // `a>b^c>d`: after climbing, `>` nests below the landing chain.
        assert_eq!(
            parsed("a>b^c>d"),
            vec![
                AbbrNode {
                    children: vec![word("b")],
                    ..word("a")
                },
                AbbrNode {
                    children: vec![word("d")],
                    ..word("c")
                },
            ]
        );
    }

    #[test]
    fn climb_landing_chain_takes_siblings() {
        // `a>b^c+d`: both c and d are top-level siblings after the climb.
        assert_eq!(
            parsed("a>b^c+d"),
            vec![
                AbbrNode {
                    children: vec![word("b")],
                    ..word("a")
                },
                word("c"),
                word("d"),
            ]
        );
    }

    #[test]
    fn climb_after_group_starts_at_group_level() {
        // `x>(a>b)^c`: the inlined group sits at x's child level, so one
        // caret makes c a top-level sibling of x.
        assert_eq!(
            parsed("x>(a>b)^c"),
            vec![
                AbbrNode {
                    children: vec![AbbrNode {
                        children: vec![word("b")],
                        ..word("a")
                    }],
                    ..word("x")
                },
                word("c"),
            ]
        );
    }

    #[test]
    fn climb_inside_group_clamps_to_group() {
        // A caret run inside `(…)` never escapes the group's top level.
        assert_eq!(
            parsed("x>(a>b^^c)+d"),
            vec![AbbrNode {
                children: vec![
                    AbbrNode {
                        children: vec![word("b")],
                        ..word("a")
                    },
                    word("c"),
                    word("d"),
                ],
                ..word("x")
            }]
        );
    }

    #[test]
    fn climb_keeps_repeat_on_previous_element() {
        // `a>b*2^c`: the climb leaves b's repeat intact and c at the top.
        assert_eq!(
            parsed("a>b*2^c"),
            vec![
                AbbrNode {
                    children: vec![AbbrNode {
                        repeat: Repeat::Count(2),
                        ..word("b")
                    }],
                    ..word("a")
                },
                word("c"),
            ]
        );
    }

    #[test]
    fn error_leading_caret() {
        let e = parse_err("^a");
        assert_eq!(e.message, "'^' climb-up must follow an element");
        assert_eq!(e.span, 0..1);
    }

    #[test]
    fn error_caret_after_operator() {
        let e = parse_err("a>^b");
        assert_eq!(e.message, "'^' climb-up must follow an element");
        assert_eq!(e.span, 2..3);
    }

    #[test]
    fn error_dangling_caret() {
        let e = parse_err("a^");
        assert_eq!(e.message, "dangling '^'");
        assert_eq!(e.span, 1..2);

        // The whole caret run is the error span.
        let e = parse_err("a>b^^");
        assert_eq!(e.message, "dangling '^'");
        assert_eq!(e.span, 3..5);
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

    #[test]
    fn error_nested_group_distribution_over_node_cap() {
        // 10^4 nodes rendering 10^6 copies: must fail at parse time, at
        // the outermost `*n` number.
        let e = parse_err("((a*100)*100)*100");
        assert_eq!(e.message, "abbreviation too large");
        assert_eq!(e.span, 14..17);
    }

    #[test]
    fn group_distribution_at_cap_still_parses() {
        // Weighted count 10,000 sits exactly at the cap.
        let nodes = parsed("(a*100)*100");
        assert_eq!(nodes.len(), 100);
        assert!(nodes.iter().all(|n| n.repeat == Repeat::Count(100)));
    }

    #[test]
    fn nested_node_repeats_parse_without_distribution() {
        // Plain `*n` chains multiply at render time, not parse time; the
        // renderer's line budget owns that case.
        let nodes = parsed("a*100>b*100>c*100");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].repeat, Repeat::Count(100));
    }
}
