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

impl Collection {
    /// Parse a `NAME=v1,v2,…` spec into a named collection, as accepted by the
    /// CLI `--repeat` flag and the web playground. Each value may carry a
    /// member after a colon (`value:type`), which becomes the item's `type`.
    /// Empty values are skipped, so a trailing comma is harmless.
    ///
    /// # Errors
    /// Returns a message when the spec has no `=`, or an empty name.
    pub fn parse_spec(spec: &str) -> Result<(String, Self), String> {
        let (name, rest) = spec
            .split_once('=')
            .ok_or_else(|| format!("expected NAME=values, got `{spec}`"))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(format!("empty collection name in `{spec}`"));
        }
        Ok((name.to_string(), Self::parse_values(rest)))
    }

    /// Parse a `NAME.key=v1,v2,…` member spec (the CLI `--member` flag): a
    /// parallel value list to attach as member `key` of the collection `NAME`,
    /// one slot per item — the data-layer "zip". Returns
    /// `(collection_name, key, slots)`; apply the slots with
    /// [`Collection::attach_member`].
    ///
    /// Slot text is taken **almost verbatim**: only *leading* whitespace is
    /// trimmed (so `a, b` reads naturally), while trailing whitespace is
    /// preserved — members are display-ready fragments, and a value like
    /// `pub ` keeps its meaningful trailing space. A slot that is empty after
    /// the leading trim means the member is **absent** for that item (the
    /// sparse escape hatch: `P.vis=pub,,pub`).
    ///
    /// # Errors
    /// Returns a message when the spec has no `=`, the left side has no `.`,
    /// the collection name is empty, or the key is empty or not an identifier
    /// (`${value.key}` can only reference identifier keys).
    pub fn parse_member_spec(spec: &str) -> Result<(String, String, Vec<Option<String>>), String> {
        let (target, values) = spec
            .split_once('=')
            .ok_or_else(|| format!("expected NAME.key=values, got `{spec}`"))?;
        let (name, key) = target
            .split_once('.')
            .ok_or_else(|| format!("expected NAME.key before `=`, got `{target}`"))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(format!("empty collection name in `{spec}`"));
        }
        let key = key.trim();
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!(
                "member key must be an identifier (as in `NAME.key=…`), got `{key}`"
            ));
        }
        let slots = values
            .split(',')
            .map(|slot| {
                let slot = slot.trim_start();
                (!slot.is_empty()).then(|| slot.to_string())
            })
            .collect();
        Ok((name.to_string(), key.to_string(), slots))
    }

    /// Attach a parallel value list as member `key` of this collection's
    /// items, one slot per item in order. A `None` slot leaves the member
    /// absent on that item (a `${value.key:default}` reference falls back to
    /// a tabstop).
    ///
    /// # Errors
    /// The slot count must equal the item count **exactly** (write an empty
    /// slot, not nothing, to skip an item); a mismatch is an error, because
    /// where this is used the data is explicit and a length drift is almost
    /// always a bug in the producing script.
    pub fn attach_member(&mut self, key: &str, slots: Vec<Option<String>>) -> Result<(), String> {
        if slots.len() != self.items.len() {
            return Err(format!(
                "{} value(s) for {} item(s); the counts must match exactly (use an empty slot to skip an item)",
                slots.len(),
                self.items.len()
            ));
        }
        for (item, slot) in self.items.iter_mut().zip(slots) {
            if let Some(value) = slot {
                item.members.insert(key.to_string(), value);
            }
        }
        Ok(())
    }

    /// Parse the `v1,v2,…` value list of a spec (the part after `=`, and the
    /// body of an inline `*[…]` literal). Each value may carry a `:type` member;
    /// empty values are skipped, so a trailing comma is harmless.
    #[must_use]
    pub fn parse_values(values: &str) -> Self {
        let items = values
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| match value.split_once(':') {
                Some((v, ty)) => {
                    let mut item = Item::new(v.trim());
                    item.members
                        .insert("type".to_string(), ty.trim().to_string());
                    item
                }
                None => Item::new(value),
            })
            .collect();
        Self { items }
    }
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

/// A ready-to-apply code-generation edit the provider found at the cursor —
/// the backing for editor code actions (e.g. "fill match arms from an enum").
///
/// It is a plain text edit (no tabstops), so any LSP client can apply it
/// without cooperating with the codemmet plugin: replace `range` with
/// `new_text`. The provider does all the work; the title is display-ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillTarget {
    /// Menu title, e.g. `"Fill match arms from Msg"`.
    pub title: String,
    /// Byte range to replace; an empty range is a pure insertion.
    pub range: Range<usize>,
    /// Replacement text, already indented for its insertion point.
    pub new_text: String,
}

/// Supplies buffer/syntax awareness to the expansion engine.
pub trait ContextProvider {
    /// Describe the expansion point at `offset` (a byte offset into the doc).
    fn context_at(&mut self, doc: &DocSnapshot, offset: usize) -> ExpansionContext;

    /// Validate (and possibly snap) a selection for wrap-with-abbreviation.
    fn validate_wrap(&mut self, doc: &DocSnapshot, range: Range<usize>) -> WrapSelection;

    /// Code-generation edits available at `offset` — the source of editor code
    /// actions. Defaults to none; the tree-sitter provider fills it in (e.g.
    /// completing a `match` from an in-scope enum).
    fn fill_targets(&mut self, _doc: &DocSnapshot, _offset: usize) -> Vec<FillTarget> {
        Vec::new()
    }
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
    fn parse_spec_reads_name_values_and_type_members() {
        let (name, collection) = Collection::parse_spec("Point=x:i32, y :f64 , z").expect("valid");
        assert_eq!(name, "Point");
        let values: Vec<&str> = collection.items.iter().map(|i| i.value.as_str()).collect();
        assert_eq!(values, ["x", "y", "z"]);
        assert_eq!(collection.items[0].members["type"], "i32");
        assert_eq!(collection.items[1].members["type"], "f64");
        assert!(collection.items[2].members.is_empty());
    }

    #[test]
    fn parse_spec_skips_empty_values_and_rejects_malformed() {
        // Trailing comma leaves no empty item.
        let (_, collection) = Collection::parse_spec("E=A,B,").expect("valid");
        assert_eq!(collection.items.len(), 2);
        // No `=`, and an empty name, are both errors.
        assert!(Collection::parse_spec("nope").is_err());
        assert!(Collection::parse_spec("=A,B").is_err());
    }

    #[test]
    fn parse_member_spec_trims_leading_and_keeps_trailing_whitespace() {
        // Q2 decision: leading trim only. `pub ` keeps its display-ready
        // trailing space; an empty slot means "absent".
        let (name, key, slots) =
            Collection::parse_member_spec("P.vis=pub , ,pub(crate) ").expect("valid");
        assert_eq!(name, "P");
        assert_eq!(key, "vis");
        assert_eq!(
            slots,
            [
                Some("pub ".to_string()),
                None,
                Some("pub(crate) ".to_string())
            ]
        );
    }

    #[test]
    fn parse_member_spec_rejects_malformed() {
        // No `=`, no `.`, empty name, empty key, non-identifier key.
        assert!(Collection::parse_member_spec("P.vis").is_err());
        assert!(Collection::parse_member_spec("Pvis=a,b").is_err());
        assert!(Collection::parse_member_spec(".vis=a,b").is_err());
        assert!(Collection::parse_member_spec("P.=a,b").is_err());
        assert!(Collection::parse_member_spec("P.a-b=x,y").is_err());
    }

    #[test]
    fn attach_member_zips_slots_onto_items() {
        let (_, mut collection) = Collection::parse_spec("P=host,port").expect("valid");
        collection
            .attach_member("type", vec![Some("String".into()), Some("u16".into())])
            .expect("lengths match");
        assert_eq!(collection.items[0].members["type"], "String");
        assert_eq!(collection.items[1].members["type"], "u16");
    }

    #[test]
    fn attach_member_sparse_slot_leaves_member_absent() {
        let (_, mut collection) = Collection::parse_spec("P=a,b,c").expect("valid");
        collection
            .attach_member("vis", vec![Some("pub ".into()), None, Some("pub ".into())])
            .expect("lengths match");
        assert!(collection.items[0].members.contains_key("vis"));
        assert!(!collection.items[1].members.contains_key("vis"));
        assert!(collection.items[2].members.contains_key("vis"));
    }

    #[test]
    fn attach_member_rejects_length_mismatch() {
        let (_, mut collection) = Collection::parse_spec("P=a,b,c").expect("valid");
        let err = collection
            .attach_member("type", vec![Some("i32".into())])
            .expect_err("too few slots");
        assert!(err.contains("1 value(s) for 3 item(s)"), "got: {err}");
        let err = collection
            .attach_member("type", vec![None; 4])
            .expect_err("too many slots");
        assert!(err.contains("4 value(s) for 3 item(s)"), "got: {err}");
    }

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
