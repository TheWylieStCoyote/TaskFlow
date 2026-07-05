//! Tree-sitter-backed [`ContextProvider`] for codemmet.
//!
//! Grammars are compiled in behind per-language cargo features
//! (`lang-rust`, `lang-python`, both on by default). For any language
//! without a compiled-in grammar — or when parsing fails — the provider
//! degrades to the exact behavior of
//! [`codemmet_core::context::NullContextProvider`].
//!
//! Parsed trees are cached per language keyed by a hash of the document
//! text, so repeated queries against an unchanged buffer are free. Full
//! reparse on change is fine at M3 scale; incremental sync via
//! `InputEdit` is deliberately deferred.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use codemmet_core::context::{
    DocSnapshot, ExpansionContext, IndentUnit, NullContextProvider, Scope, WrapSelection,
};
use tree_sitter::{Node, Tree};

pub use codemmet_core::context::ContextProvider;

/// Whether this build carries a grammar for `language_id`.
///
/// `language_id` values are canonical profile ids (`"rust"`, `"python"`).
pub fn supports(language_id: &str) -> bool {
    language_for(language_id).is_some()
}

/// Grammar registry: canonical language id → compiled-in grammar.
///
/// (`const` is only possible in the degenerate zero-feature build, so the
/// lint is silenced for that configuration.)
#[cfg_attr(
    not(any(feature = "lang-rust", feature = "lang-python")),
    allow(clippy::missing_const_for_fn)
)]
fn language_for(id: &str) -> Option<tree_sitter::Language> {
    match id {
        #[cfg(feature = "lang-rust")]
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        #[cfg(feature = "lang-python")]
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        _ => None,
    }
}

/// Map a raw node kind to a normalized [`Scope`] for `language`.
///
/// Kinds absent from the table contribute only to `raw_node_kinds`;
/// [`Scope::Unknown`] is never produced.
fn scope_for_kind(language: &str, kind: &str) -> Option<Scope> {
    match language {
        "rust" => match kind {
            "impl_item" | "trait_item" => Some(Scope::TypeBody),
            "function_item" | "closure_expression" => Some(Scope::FnBody),
            "for_expression" | "while_expression" | "loop_expression" => Some(Scope::Loop),
            "if_expression" => Some(Scope::Conditional),
            "match_expression" => Some(Scope::MatchBody),
            _ => None,
        },
        "python" => match kind {
            "class_definition" => Some(Scope::TypeBody),
            "function_definition" => Some(Scope::FnBody),
            "for_statement" | "while_statement" => Some(Scope::Loop),
            "if_statement" => Some(Scope::Conditional),
            "match_statement" => Some(Scope::MatchBody),
            "try_statement" => Some(Scope::TryBody),
            _ => None,
        },
        _ => None,
    }
}

/// String-literal node kinds per language (comments are matched by kind
/// substring instead, which also covers doc comments).
fn is_string_kind(language: &str, kind: &str) -> bool {
    match language {
        "rust" => matches!(
            kind,
            "string_literal" | "raw_string_literal" | "char_literal"
        ),
        "python" => matches!(kind, "string" | "string_content" | "concatenated_string"),
        _ => false,
    }
}

/// Start byte of the body/block child of a scope-bearing node, if present.
///
/// Most constructs in both grammars expose their suite via a `body` field;
/// `if` constructs use `consequence`. As a last resort scan for a
/// block-like named child. `None` means the body is missing entirely
/// (half-typed construct), which callers treat as "already inside".
fn body_start(node: Node<'_>) -> Option<usize> {
    for field in ["body", "consequence"] {
        if let Some(child) = node.child_by_field_name(field) {
            return Some(child.start_byte());
        }
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|c| matches!(c.kind(), "block" | "declaration_list" | "match_block"))
        .map(|c| c.start_byte())
}

/// Clamp a byte offset into `text`, snapping down to a char boundary.
fn clamp_offset(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// First indented, non-blank line decides the indent unit; `Spaces(4)`
/// when the document has no indented lines yet.
fn detect_indent_unit(text: &str) -> IndentUnit {
    for line in text.lines() {
        let content = line.trim_start_matches([' ', '\t']);
        if content.is_empty() || content.len() == line.len() {
            continue;
        }
        let ws = &line[..line.len() - content.len()];
        if ws.starts_with('\t') {
            return IndentUnit::Tab;
        }
        return IndentUnit::Spaces(u8::try_from(ws.len()).unwrap_or(u8::MAX));
    }
    IndentUnit::Spaces(4)
}

/// Tree-sitter-backed [`ContextProvider`] with a per-language tree cache.
#[derive(Debug, Default)]
pub struct TreeSitterProvider {
    /// language id → (hash of the text the tree was parsed from, tree).
    cache: HashMap<String, (u64, Tree)>,
    /// Number of full parses performed; cache hits do not parse.
    parses: u64,
}

impl TreeSitterProvider {
    /// Create a provider with an empty tree cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many full parses this provider has run (cache diagnostics).
    pub const fn parse_count(&self) -> u64 {
        self.parses
    }

    /// Parse `doc` (or reuse the cached tree when language + text match).
    ///
    /// `None` for unsupported languages or parser failure.
    fn tree_for(&mut self, doc: &DocSnapshot) -> Option<Tree> {
        let language = language_for(&doc.language)?;
        let mut hasher = DefaultHasher::new();
        doc.text.hash(&mut hasher);
        let hash = hasher.finish();
        if let Some((cached_hash, tree)) = self.cache.get(&doc.language)
            && *cached_hash == hash
        {
            return Some(tree.clone());
        }
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).ok()?;
        let tree = parser.parse(&doc.text, None)?;
        self.parses += 1;
        self.cache
            .insert(doc.language.clone(), (hash, tree.clone()));
        Some(tree)
    }
}

impl ContextProvider for TreeSitterProvider {
    fn context_at(&mut self, doc: &DocSnapshot, offset: usize) -> ExpansionContext {
        let Some(tree) = self.tree_for(doc) else {
            return NullContextProvider.context_at(doc, offset);
        };
        let text = doc.text.as_str();
        let offset = clamp_offset(text, offset);
        let root = tree.root_node();

        // Ancestor chain covering the cursor gap, leaf up to root, then
        // reversed so both output vectors run outermost → innermost.
        let mut chain = Vec::new();
        let mut node = root
            .named_descendant_for_byte_range(offset, offset)
            .unwrap_or(root);
        loop {
            chain.push(node);
            match node.parent() {
                Some(parent) => node = parent,
                None => break,
            }
        }
        chain.reverse();

        let mut raw_node_kinds = Vec::new();
        let mut scopes = vec![Scope::Root];
        let mut in_string_or_comment = false;
        for node in chain {
            // Half-typed trigger words produce ERROR/missing nodes; skip
            // them and keep climbing so real enclosing scopes still count.
            if node.is_error() || node.is_missing() || !node.is_named() {
                continue;
            }
            let kind = node.kind();
            raw_node_kinds.push(kind.to_string());
            // A cursor sitting exactly before the opening quote/delimiter
            // (offset == start_byte) is not "inside" the literal.
            if offset > node.start_byte()
                && (kind.contains("comment") || is_string_kind(&doc.language, kind))
            {
                in_string_or_comment = true;
            }
            if let Some(scope) = scope_for_kind(&doc.language, kind) {
                // The scope only counts once the cursor is inside the
                // construct's body, not its header/signature. A missing
                // body (broken parse mid-typing) counts anyway.
                if body_start(node).is_none_or(|body| offset >= body) {
                    scopes.push(scope);
                }
            }
        }

        let indent_unit = detect_indent_unit(text);
        let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[offset..].find('\n').map_or(text.len(), |i| offset + i);
        let line = &text[line_start..line_end];
        let base_indent = if line.trim_matches([' ', '\t']).is_empty() {
            // Blank line: derive indentation from tree depth (one level
            // per entered body scope) instead of the line's whitespace.
            indent_unit.as_str().repeat(scopes.len() - 1)
        } else {
            line.chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect()
        };

        ExpansionContext {
            scopes,
            raw_node_kinds,
            in_string_or_comment,
            base_indent,
            indent_unit,
        }
    }

    fn validate_wrap(&mut self, doc: &DocSnapshot, range: Range<usize>) -> WrapSelection {
        if !supports(&doc.language) {
            return WrapSelection::Valid { range };
        }
        // M3 scope: snap the selection outward to whole-line boundaries.
        // Statement-level snapping and cross-scope rejection land in M4.
        let text = doc.text.as_str();
        let start = clamp_offset(text, range.start);
        let end = clamp_offset(text, range.end.max(range.start));
        let start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        let end = text[end..].find('\n').map_or(text.len(), |i| end + i);
        WrapSelection::Valid { range: start..end }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Split a fixture on the `§` cursor marker → (doc, byte offset).
    fn doc_at(language: &str, fixture: &str) -> (DocSnapshot, usize) {
        let offset = fixture.find('§').expect("fixture contains cursor marker");
        let text = fixture.replacen('§', "", 1);
        (
            DocSnapshot {
                language: language.into(),
                text,
            },
            offset,
        )
    }

    #[cfg(any(feature = "lang-rust", feature = "lang-python"))]
    fn ctx(language: &str, fixture: &str) -> ExpansionContext {
        let (doc, offset) = doc_at(language, fixture);
        TreeSitterProvider::new().context_at(&doc, offset)
    }

    #[cfg(feature = "lang-rust")]
    mod rust {
        use super::*;

        #[test]
        fn top_level_is_root_only() {
            let c = ctx("rust", "§\nfn main() {}\n");
            assert_eq!(c.scopes, vec![Scope::Root]);
            assert_eq!(c.raw_node_kinds, vec!["source_file"]);
            assert!(!c.in_string_or_comment);
            assert_eq!(c.base_indent, "");
        }

        #[test]
        fn impl_body_is_type_body() {
            let c = ctx("rust", "impl Point {\n    §\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody]);
            assert_eq!(
                c.raw_node_kinds,
                vec!["source_file", "impl_item", "declaration_list"]
            );
            assert!(!c.in_string_or_comment);
        }

        #[test]
        fn fn_inside_impl_stacks_scopes() {
            let c = ctx(
                "rust",
                "impl Point {\n    fn norm(&self) {\n        §\n    }\n}\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody, Scope::FnBody]);
            assert_eq!(c.base_indent, "        ");
            assert_eq!(c.indent_unit, IndentUnit::Spaces(4));
        }

        #[test]
        fn half_typed_fn_keeps_enclosing_type_body() {
            // "fn" alone produces ERROR/missing nodes; the walk must skip
            // them and still report the enclosing impl body.
            let c = ctx("rust", "impl Point {\n    fn§\n}\n");
            assert!(
                c.scopes.starts_with(&[Scope::Root, Scope::TypeBody]),
                "scopes = {:?}",
                c.scopes
            );
            assert!(!c.in_string_or_comment);
        }

        #[test]
        fn for_body_is_loop() {
            let c = ctx(
                "rust",
                "fn main() {\n    for i in 0..10 {\n        §\n    }\n}\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody, Scope::Loop]);
        }

        #[test]
        fn match_body_is_match() {
            let c = ctx("rust", "fn main() {\n    match x {\n        §\n    }\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody, Scope::MatchBody]);
        }

        #[test]
        fn inside_string_literal() {
            let c = ctx("rust", "fn main() { let s = \"he§llo\"; }\n");
            assert!(c.in_string_or_comment);
        }

        #[test]
        fn before_opening_quote_is_not_in_string() {
            let c = ctx("rust", "fn main() { let s = §\"hello\"; }\n");
            assert!(!c.in_string_or_comment);
        }

        #[test]
        fn inside_line_comment() {
            let c = ctx("rust", "fn main() {\n    // he§llo\n}\n");
            assert!(c.in_string_or_comment);
        }

        #[test]
        fn inside_doc_comment() {
            let c = ctx("rust", "/// do§cs\nfn main() {}\n");
            assert!(c.in_string_or_comment);
        }

        #[test]
        fn impl_header_is_not_type_body() {
            let c = ctx("rust", "impl Po§int {}\n");
            assert_eq!(c.scopes, vec![Scope::Root]);
        }

        #[test]
        fn blank_line_indent_comes_from_tree() {
            // The cursor line is completely empty; indentation must come
            // from tree depth (one body scope), not the line prefix.
            let c = ctx("rust", "impl Point {\n§\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody]);
            assert_eq!(c.base_indent, "    ");
        }

        #[test]
        fn indent_unit_tab() {
            let c = ctx("rust", "fn main() {\n\tlet x = 1;\n\t§\n}\n");
            assert_eq!(c.indent_unit, IndentUnit::Tab);
            assert_eq!(c.base_indent, "\t");
        }

        #[test]
        fn indent_unit_two_spaces() {
            let c = ctx("rust", "fn main() {\n  let x = 1;\n  §\n}\n");
            assert_eq!(c.indent_unit, IndentUnit::Spaces(2));
            assert_eq!(c.base_indent, "  ");
        }

        #[test]
        fn indent_unit_four_spaces() {
            let c = ctx("rust", "fn main() {\n    let x = 1;\n    §\n}\n");
            assert_eq!(c.indent_unit, IndentUnit::Spaces(4));
            assert_eq!(c.base_indent, "    ");
        }

        #[test]
        fn wrap_snaps_to_line_boundaries() {
            let doc = DocSnapshot {
                language: "rust".into(),
                text: "fn main() {\n    let a = 1;\n    let b = 2;\n}\n".into(),
            };
            let a_line = doc.text.find("    let a").expect("fixture line");
            let b_line = doc.text.find("    let b").expect("fixture line");
            let b_end = doc.text[b_line..]
                .find('\n')
                .map(|i| b_line + i)
                .expect("fixture newline");
            // Selection starts mid-`let a` line, ends mid-`let b` line.
            let sel = (a_line + 6)..(b_line + 6);
            match TreeSitterProvider::new().validate_wrap(&doc, sel) {
                WrapSelection::Valid { range } => assert_eq!(range, a_line..b_end),
                WrapSelection::Invalid { reason } => panic!("unexpected invalid: {reason}"),
            }
        }

        #[test]
        fn cache_reuses_tree_for_unchanged_text() {
            let (doc, offset) = doc_at("rust", "impl Point {\n    §\n}\n");
            let mut provider = TreeSitterProvider::new();
            provider.context_at(&doc, offset);
            provider.context_at(&doc, offset);
            assert_eq!(provider.parse_count(), 1);
            let changed = DocSnapshot {
                language: doc.language.clone(),
                text: format!("{}\n// more\n", doc.text),
            };
            provider.context_at(&changed, offset);
            assert_eq!(provider.parse_count(), 2);
        }
    }

    #[cfg(feature = "lang-python")]
    mod python {
        use super::*;

        #[test]
        fn class_body_is_type_body() {
            let c = ctx("python", "class Foo:\n    x = 1\n    §\n    y = 2\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody]);
            assert_eq!(c.base_indent, "    ");
            assert_eq!(c.indent_unit, IndentUnit::Spaces(4));
        }

        #[test]
        fn def_in_class_stacks_scopes() {
            let c = ctx(
                "python",
                "class Foo:\n    def bar(self):\n        x = 1\n        §\n        y = 2\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody, Scope::FnBody]);
            assert_eq!(c.base_indent, "        ");
        }

        #[test]
        fn try_body_is_try() {
            let c = ctx(
                "python",
                "try:\n    x = 1\n    §\n    y = 2\nexcept ValueError:\n    pass\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TryBody]);
        }

        #[test]
        fn inside_string() {
            let c = ctx("python", "x = \"he§llo\"\n");
            assert!(c.in_string_or_comment);
        }

        #[test]
        fn inside_comment() {
            let c = ctx("python", "# he§llo\nx = 1\n");
            assert!(c.in_string_or_comment);
        }
    }

    #[test]
    fn unsupported_language_matches_null_provider() {
        let (doc, offset) = doc_at("go", "func main() {\n\t§x\n}\n");
        let ts = TreeSitterProvider::new().context_at(&doc, offset);
        let null = NullContextProvider.context_at(&doc, offset);
        assert_eq!(ts.scopes, null.scopes);
        assert_eq!(ts.raw_node_kinds, null.raw_node_kinds);
        assert_eq!(ts.in_string_or_comment, null.in_string_or_comment);
        assert_eq!(ts.base_indent, null.base_indent);
        assert_eq!(ts.indent_unit, null.indent_unit);
    }

    #[test]
    fn wrap_passes_unsupported_language_through() {
        let doc = DocSnapshot {
            language: "go".into(),
            text: "func main() {\n\tx := 1\n}\n".into(),
        };
        match TreeSitterProvider::new().validate_wrap(&doc, 16..19) {
            WrapSelection::Valid { range } => assert_eq!(range, 16..19),
            WrapSelection::Invalid { reason } => panic!("unexpected invalid: {reason}"),
        }
    }
}
