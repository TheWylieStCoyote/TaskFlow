//! Recursive-descent parser for the abbreviation mini-grammar.

use super::{Abbreviation, ParseError};

/// Parse an abbreviation like `fn:main>fori10>pr` into its AST.
pub fn parse(input: &str) -> Result<Abbreviation, ParseError> {
    let _ = input;
    todo!("M1: abbreviation lexer + recursive-descent parser")
}
