//! The abbreviation mini-grammar: `fn:main>fori10>pr`.
//!
//! Grammar (see `PLAN.md` for the full spec):
//!
//! ```text
//! abbr     := group
//! group    := level ( '^'+ level )*        ; '^' climbs, clamped at this group
//! level    := chain ( '+' chain )*         ; siblings
//! chain    := element ( '>' level )?       ; '>' attaches following level as children
//! element  := atom ( '*' NUMBER )?
//! atom     := node | '(' group ')'
//! node     := WORD args?
//! args     := ':' arg ( ',' arg )*
//! ```
//!
//! `^` (climb-up) ends the current `>`-nesting level and attaches the
//! following chain N levels up, where N is the number of consecutive carets:
//! `a>b^c` puts `c` back at the top level, `a>b>c^d` makes `d` a sibling of
//! `b`. Carets beyond the top level clamp there (`a^^^b` is `a+b`).
//!
//! Groups interact with `^` per the flat-distribution design (a `(…)` group
//! is inlined into its surrounding level, [`AbbrNode`] has no group concept):
//!
//! - **After** a group, `^` climbs from the level the group occupies:
//!   `x>(a>b)^c` puts the group at `x`'s child level, so one caret makes `c`
//!   a top-level sibling of `x`.
//! - **Inside** a group, `^` clamps at the group's own top level and never
//!   escapes the parentheses: `x>(a>b^^c)` keeps `c` inside the group, as a
//!   sibling of `a` (both children of `x`).
//!
//! The parser is language-agnostic: it does **not** split `fori10` into
//! trigger + glued arg. That resolution needs the active language's trigger
//! set and happens in the renderer via
//! [`SnippetSet::resolve_word`](crate::config::loader::SnippetSet::resolve_word).

mod parser;

pub use parser::parse;

use std::ops::Range;

/// Canonical set of operator characters on the abbreviation surface — the
/// single source of truth for code that scans abbreviation text without
/// parsing it (candidate extraction, operator gating).
///
/// Mirrors the grammar above: `>` child, `+` sibling, `*` repeat, `:` args,
/// `,` arg separator, `(`/`)` groups, `^` climb-up.
pub const OPERATOR_CHARS: &[char] = &['>', '+', '*', ':', ',', '(', ')', '^'];

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
    pub children: Vec<Self>,
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
