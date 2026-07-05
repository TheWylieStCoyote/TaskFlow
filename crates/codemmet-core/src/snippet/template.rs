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

use super::ir::Template;

/// A template body that violates the schema rules.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct TemplateError(pub String);

/// Parse a snippet `body` string into the line/span IR.
pub fn parse_template(body: &str) -> Result<Template, TemplateError> {
    let _ = body;
    todo!("M1: template body parser")
}
