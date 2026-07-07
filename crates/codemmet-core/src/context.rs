//! The seam between the pure expansion engine and editor/syntax awareness.
//!
//! `codemmet-core` never talks to tree-sitter directly. Anything that needs
//! knowledge of the buffer (enclosing scopes, indentation, wrap boundaries)
//! is answered by a [`ContextProvider`]. Phase 1 ships only
//! [`NullContextProvider`]; the `codemmet-tsctx` crate supplies a
//! tree-sitter-backed implementation in later phases.

use std::collections::BTreeMap;
use std::ops::Range;

/// One member of a [`Collection`] that drives data-driven repetition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Item {
    /// The item's primary text — a `${value}` reference renders this (an enum
    /// variant name, a list element, a struct field name).
    pub value: String,
    /// Named extras a `${value.key}` reference can render (e.g. a field's
    /// `"type"`).
    pub members: BTreeMap<String, String>,
}

impl Item {
    /// A member-less item carrying just a primary value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            members: BTreeMap::new(),
        }
    }
}

/// A named, ordered collection resolved from the buffer (or supplied on the
/// CLI) that a data-driven repeat (`arm*@Msg`, or a bare `*` over a subject)
/// expands over — one copy per item.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Collection {
    /// The items, in source order.
    pub items: Vec<Item>,
}

/// A read-only snapshot of the document an expansion happens in.
#[derive(Debug, Clone)]
pub struct DocSnapshot {
    /// Language identifier, e.g. `"rust"` or `"python"`.
    pub language: String,
    /// Full document text.
    pub text: String,
}

/// The indentation unit to render with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndentUnit {
    /// Indent with `n` spaces per level.
    Spaces(u8),
    /// Indent with one tab per level.
    Tab,
}

impl IndentUnit {
    /// The literal text for one level of indentation.
    pub fn as_str(&self) -> String {
        match self {
            Self::Spaces(n) => " ".repeat(usize::from(*n)),
            Self::Tab => "\t".to_string(),
        }
    }
}

/// Normalized syntactic scope surrounding the expansion point.
///
/// Snippet variants match on these (e.g. a `fn` trigger inside
/// [`Scope::TypeBody`] can render with `&self`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Scope {
    /// Top level of the file.
    Root,
    /// Body of a type-like construct (`impl`/`trait` in Rust, `class` bodies).
    TypeBody,
    /// Body of a function or method.
    FnBody,
    /// Body of a loop.
    Loop,
    /// Body of an `if`/`else`-like construct.
    Conditional,
    /// Body of a `match`/`switch`.
    MatchBody,
    /// Body of a `try`-like construct.
    TryBody,
    /// A scope the provider could not classify.
    Unknown,
}

/// Everything the renderer needs to know about the expansion point.
#[derive(Debug, Clone)]
pub struct ExpansionContext {
    /// Enclosing scopes, outermost first, innermost last.
    pub scopes: Vec<Scope>,
    /// Raw syntax-node kinds for `ts:`-prefixed variant matching.
    pub raw_node_kinds: Vec<String>,
    /// Whether the cursor sits inside a string literal or comment.
    pub in_string_or_comment: bool,
    /// Leading whitespace of the line the expansion starts on.
    pub base_indent: String,
    /// Indentation unit for nested lines.
    pub indent_unit: IndentUnit,
    /// Named collections in scope, for data-driven repeats (`arm*@Msg`, or a
    /// bare `*` over a subject). Empty when there is no context data.
    pub collections: BTreeMap<String, Collection>,
}

impl ExpansionContext {
    /// A context with no syntactic knowledge: root scope, given indentation.
    pub fn plain(base_indent: String, indent_unit: IndentUnit) -> Self {
        Self {
            scopes: vec![Scope::Root],
            raw_node_kinds: Vec::new(),
            in_string_or_comment: false,
            base_indent,
            indent_unit,
            collections: BTreeMap::new(),
        }
    }
}

/// Result of validating a wrap-with-abbreviation selection.
#[derive(Debug, Clone)]
pub enum WrapSelection {
    /// The selection (possibly snapped outward to statement boundaries).
    Valid {
        /// Byte range of the text to wrap after snapping.
        range: Range<usize>,
    },
    /// The selection cannot be wrapped; the message explains why.
    Invalid {
        /// Human-readable reason suitable for an editor notification.
        reason: String,
    },
}

/// Supplies buffer/syntax awareness to the expansion engine.
pub trait ContextProvider {
    /// Describe the expansion point at `offset` (a byte offset into the doc).
    fn context_at(&mut self, doc: &DocSnapshot, offset: usize) -> ExpansionContext;

    /// Validate (and possibly snap) a selection for wrap-with-abbreviation.
    fn validate_wrap(&mut self, doc: &DocSnapshot, range: Range<usize>) -> WrapSelection;
}

/// Syntax-unaware provider used until tree-sitter support lands (M3).
///
/// Indentation is inferred from the line containing `offset`, falling
/// back to the language profile's default indent (then four spaces for
/// unknown languages); scope is always [`Scope::Root`]; every wrap
/// selection is accepted as-is.
#[derive(Debug, Default)]
pub struct NullContextProvider;

impl ContextProvider for NullContextProvider {
    fn context_at(&mut self, doc: &DocSnapshot, offset: usize) -> ExpansionContext {
        let line_start = doc.text[..offset.min(doc.text.len())]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let base_indent: String = doc.text[line_start..]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let indent_unit = if base_indent.starts_with('\t') {
            IndentUnit::Tab
        } else {
            // Nothing detected on the line: fall back to the language
            // profile's default, then to four spaces for unknown languages.
            crate::lang::profile::find(&doc.language)
                .map_or(IndentUnit::Spaces(4), |p| p.default_indent.clone())
        };
        ExpansionContext::plain(base_indent, indent_unit)
    }

    fn validate_wrap(&mut self, _doc: &DocSnapshot, range: Range<usize>) -> WrapSelection {
        WrapSelection::Valid { range }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_provider_takes_indent_from_current_line() {
        let doc = DocSnapshot {
            language: "rust".into(),
            text: "fn main() {\n    fori\n}\n".into(),
        };
        let offset = doc.text.find("fori").expect("fixture contains trigger");
        let ctx = NullContextProvider.context_at(&doc, offset);
        assert_eq!(ctx.base_indent, "    ");
        assert_eq!(ctx.indent_unit, IndentUnit::Spaces(4));
        assert_eq!(ctx.scopes, vec![Scope::Root]);
    }

    #[test]
    fn null_provider_flat_go_doc_uses_profile_tab() {
        let doc = DocSnapshot {
            language: "go".into(),
            text: "package main\n\n".into(),
        };
        let ctx = NullContextProvider.context_at(&doc, doc.text.len());
        assert_eq!(ctx.indent_unit, IndentUnit::Tab);
        assert_eq!(ctx.base_indent, "");
    }

    #[test]
    fn null_provider_unknown_language_defaults_to_four_spaces() {
        let doc = DocSnapshot {
            language: "cobol".into(),
            text: "x = 1\n".into(),
        };
        let ctx = NullContextProvider.context_at(&doc, doc.text.len());
        assert_eq!(ctx.indent_unit, IndentUnit::Spaces(4));
    }

    #[test]
    fn null_provider_defaults_at_offset_zero() {
        let doc = DocSnapshot {
            language: "python".into(),
            text: "def f():\n\tpass\n".into(),
        };
        let ctx = NullContextProvider.context_at(&doc, 0);
        assert_eq!(ctx.base_indent, "");
        assert!(!ctx.in_string_or_comment);
    }
}
