//! Lexer (tokenizer) for the filter DSL.
//!
//! Converts input strings into a stream of tokens for the parser.

use std::sync::LazyLock;

use regex::Regex;

use super::error::{ParseError, ParseResult};

/// Pre-compiled regex for identifiers (field names and values).
/// Allows letters, digits, underscores, hyphens, and dots. Can start with letter, digit, underscore, or dots.
/// Also allows optional `<` or `>` prefix for comparison values (e.g., `<2025-12-25`).
/// Dots are allowed for range syntax:
/// - Full range: `2025-01-01..2025-12-31`
/// - Open start: `..2025-12-31` (up to end)
/// - Open end: `2025-01-01..` (from start onward)
static IDENTIFIER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[<>]?\.{0,2}[a-zA-Z0-9_][a-zA-Z0-9_.\-]*").expect("valid regex")
});

/// Token types produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// An identifier (field name or unquoted value).
    Identifier(String),

    /// A quoted string value.
    QuotedString(String),

    /// The AND operator.
    And,

    /// The OR operator.
    Or,

    /// The NOT operator (!).
    Not,

    /// Colon separator (:).
    Colon,

    /// Left parenthesis.
    LParen,

    /// Right parenthesis.
    RParen,

    /// End of input.
    Eof,
}

impl Token {
    /// Get a display name for the token (for error messages).
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Identifier(_) => "identifier",
            Self::QuotedString(_) => "quoted string",
            Self::And => "AND",
            Self::Or => "OR",
            Self::Not => "!",
            Self::Colon => ":",
            Self::LParen => "(",
            Self::RParen => ")",
            Self::Eof => "end of input",
        }
    }
}

/// A token with its position in the input.
#[derive(Debug, Clone)]
pub struct TokenWithSpan {
    /// The token.
    pub token: Token,
    /// Start position in the input.
    pub start: usize,
    /// End position in the input (reserved for future span-based error reporting).
    #[allow(dead_code)]
    pub end: usize,
}

impl TokenWithSpan {
    /// Create a new token with span.
    pub fn new(token: Token, start: usize, end: usize) -> Self {
        Self { token, start, end }
    }
}

/// Lexer for tokenizing filter DSL input.
pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input.
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    /// Tokenize the entire input and return a vector of tokens.
    pub fn tokenize(&mut self) -> ParseResult<Vec<TokenWithSpan>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            if self.position >= self.input.len() {
                tokens.push(TokenWithSpan::new(Token::Eof, self.position, self.position));
                break;
            }

            tokens.push(self.next_token()?);
        }

        Ok(tokens)
    }

    /// Get the next token from the input.
    fn next_token(&mut self) -> ParseResult<TokenWithSpan> {
        let start = self.position;
        let remaining = &self.input[self.position..];

        // Check single-character tokens first
        if let Some(ch) = remaining.chars().next() {
            match ch {
                '(' => {
                    self.position += 1;
                    return Ok(TokenWithSpan::new(Token::LParen, start, self.position));
                }
                ')' => {
                    self.position += 1;
                    return Ok(TokenWithSpan::new(Token::RParen, start, self.position));
                }
                ':' => {
                    self.position += 1;
                    return Ok(TokenWithSpan::new(Token::Colon, start, self.position));
                }
                '!' => {
                    self.position += 1;
                    return Ok(TokenWithSpan::new(Token::Not, start, self.position));
                }
                '"' => {
                    return self.lex_quoted_string(start);
                }
                _ => {}
            }
        }

        // Check for identifiers and keywords
        if let Some(caps) = IDENTIFIER_RE.captures(remaining) {
            let matched = &caps[0];
            self.position += matched.len();

            let token = match matched.to_uppercase().as_str() {
                "AND" => Token::And,
                "OR" => Token::Or,
                _ => Token::Identifier(matched.to_string()),
            };

            return Ok(TokenWithSpan::new(token, start, self.position));
        }

        // Unknown character
        let ch = remaining.chars().next().unwrap();
        Err(ParseError::unexpected_char(start, ch))
    }

    /// Lex a quoted string starting at the given position.
    fn lex_quoted_string(&mut self, start: usize) -> ParseResult<TokenWithSpan> {
        // Skip the opening quote
        self.position += 1;

        let content_start = self.position;
        let mut found_end = false;

        while self.position < self.input.len() {
            let ch = self.input[self.position..].chars().next().unwrap();
            if ch == '"' {
                found_end = true;
                break;
            }
            self.position += ch.len_utf8();
        }

        if !found_end {
            return Err(ParseError::unterminated_string(start));
        }

        let content = self.input[content_start..self.position].to_string();

        // Skip the closing quote
        self.position += 1;

        Ok(TokenWithSpan::new(
            Token::QuotedString(content),
            start,
            self.position,
        ))
    }

    /// Skip whitespace characters.
    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() {
            let ch = self.input[self.position..].chars().next().unwrap();
            if ch.is_whitespace() {
                self.position += ch.len_utf8();
            } else {
                break;
            }
        }
    }
}

/// Convenience function to tokenize a string.
pub fn tokenize(input: &str) -> ParseResult<Vec<TokenWithSpan>> {
    Lexer::new(input).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_tokens(input: &str) -> Vec<Token> {
        tokenize(input)
            .unwrap()
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    #[test]
    fn test_empty_input() {
        let tokens = get_tokens("");
        assert_eq!(tokens, vec![Token::Eof]);
    }

    #[test]
    fn test_whitespace_only() {
        let tokens = get_tokens("   \t\n  ");
        assert_eq!(tokens, vec![Token::Eof]);
    }

    #[test]
    fn test_simple_condition() {
        let tokens = get_tokens("priority:high");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("priority".to_string()),
                Token::Colon,
                Token::Identifier("high".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_and_operator() {
        let tokens = get_tokens("a:1 AND b:2");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".to_string()),
                Token::Colon,
                Token::Identifier("1".to_string()),
                Token::And,
                Token::Identifier("b".to_string()),
                Token::Colon,
                Token::Identifier("2".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_or_operator() {
        let tokens = get_tokens("a:1 OR b:2");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".to_string()),
                Token::Colon,
                Token::Identifier("1".to_string()),
                Token::Or,
                Token::Identifier("b".to_string()),
                Token::Colon,
                Token::Identifier("2".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_not_operator() {
        let tokens = get_tokens("!status:done");
        assert_eq!(
            tokens,
            vec![
                Token::Not,
                Token::Identifier("status".to_string()),
                Token::Colon,
                Token::Identifier("done".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_parentheses() {
        let tokens = get_tokens("(a:1 OR b:2)");
        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::Identifier("a".to_string()),
                Token::Colon,
                Token::Identifier("1".to_string()),
                Token::Or,
                Token::Identifier("b".to_string()),
                Token::Colon,
                Token::Identifier("2".to_string()),
                Token::RParen,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_quoted_string() {
        let tokens = get_tokens(r#"search:"hello world""#);
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("search".to_string()),
                Token::Colon,
                Token::QuotedString("hello world".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_quoted_string_empty() {
        let tokens = get_tokens(r#"search:"""#);
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("search".to_string()),
                Token::Colon,
                Token::QuotedString(String::new()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_unterminated_string() {
        let result = tokenize(r#"search:"unterminated"#);
        assert!(matches!(result, Err(ParseError::UnterminatedString { .. })));
    }

    #[test]
    fn test_case_insensitive_operators() {
        let tokens1 = get_tokens("a:1 and b:2");
        let tokens2 = get_tokens("a:1 AND b:2");
        let tokens3 = get_tokens("a:1 And b:2");

        // All should have And token
        assert!(tokens1.contains(&Token::And));
        assert!(tokens2.contains(&Token::And));
        assert!(tokens3.contains(&Token::And));
    }

    #[test]
    fn test_identifier_with_hyphens() {
        let tokens = get_tokens("status:in-progress");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("status".to_string()),
                Token::Colon,
                Token::Identifier("in-progress".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_identifier_with_underscores() {
        let tokens = get_tokens("status:in_progress");
        assert_eq!(
            tokens,
            vec![
                Token::Identifier("status".to_string()),
                Token::Colon,
                Token::Identifier("in_progress".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_complex_expression() {
        let tokens = get_tokens("(priority:high OR priority:urgent) AND !status:done");
        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::Identifier("priority".to_string()),
                Token::Colon,
                Token::Identifier("high".to_string()),
                Token::Or,
                Token::Identifier("priority".to_string()),
                Token::Colon,
                Token::Identifier("urgent".to_string()),
                Token::RParen,
                Token::And,
                Token::Not,
                Token::Identifier("status".to_string()),
                Token::Colon,
                Token::Identifier("done".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn test_unexpected_char() {
        let result = tokenize("priority@high");
        assert!(matches!(
            result,
            Err(ParseError::UnexpectedChar { char: '@', .. })
        ));
    }

    #[test]
    fn test_token_spans() {
        let tokens = tokenize("a:1").unwrap();
        assert_eq!(tokens[0].start, 0);
        assert_eq!(tokens[0].end, 1);
        assert_eq!(tokens[1].start, 1);
        assert_eq!(tokens[1].end, 2);
        assert_eq!(tokens[2].start, 2);
        assert_eq!(tokens[2].end, 3);
    }

    #[test]
    fn test_token_display_name() {
        assert_eq!(
            Token::Identifier("test".to_string()).display_name(),
            "identifier"
        );
        assert_eq!(
            Token::QuotedString("test".to_string()).display_name(),
            "quoted string"
        );
        assert_eq!(Token::And.display_name(), "AND");
        assert_eq!(Token::Or.display_name(), "OR");
        assert_eq!(Token::Not.display_name(), "!");
        assert_eq!(Token::Colon.display_name(), ":");
        assert_eq!(Token::LParen.display_name(), "(");
        assert_eq!(Token::RParen.display_name(), ")");
        assert_eq!(Token::Eof.display_name(), "end of input");
    }
}
