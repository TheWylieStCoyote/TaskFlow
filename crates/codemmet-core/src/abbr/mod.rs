//! The abbreviation mini-grammar: `fn:main>fori10>pr`.
//!
//! Grammar (see `PLAN.md` for the full spec):
//!
//! ```text
//! abbr     := group
//! group    := chain ( '+' chain )*         ; siblings
//! chain    := element ( '>' group )?       ; '>' attaches following group as children
//! element  := atom ( '*' NUMBER )?
//! atom     := node | '(' group ')'
//! node     := WORD args?
//! args     := ':' arg ( ',' arg )*
//! ```
//!
//! The parser is language-agnostic: it does **not** split `fori10` into
//! trigger + glued arg. That resolution needs the active language's trigger
//! set and happens in the renderer via
//! [`SnippetSet::resolve_word`](crate::config::loader::SnippetSet::resolve_word).

mod parser;

pub use parser::parse;

use std::ops::Range;

/// A parsed abbreviation: the top-level sibling chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Abbreviation(pub Vec<AbbrNode>);

/// One node of the abbreviation tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbbrNode {
    /// The raw WORD token (trigger plus any glued arg, e.g. `fori10`).
    pub word: String,
    /// Explicit `:`-args, already unquoted and unescaped, in source order.
    pub args: Vec<String>,
    /// Repetition count from `*n`; `1` when absent.
    pub repeat: u32,
    /// Nodes attached with `>`.
    pub children: Vec<AbbrNode>,
}

/// A parse failure with a byte-offset span into the abbreviation text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message} at byte {}..{}", span.start, span.end)]
pub struct ParseError {
    /// Human-readable description of the failure.
    pub message: String,
    /// Byte range in the input the error points at.
    pub span: Range<usize>,
}
