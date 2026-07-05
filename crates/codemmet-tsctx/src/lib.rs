//! Tree-sitter-backed [`ContextProvider`] for codemmet.
//!
//! Grammars are compiled in behind per-language cargo features
//! (`lang-rust`, `lang-python`, `lang-c`, `lang-cpp`, `lang-javascript`,
//! `lang-typescript`, `lang-go`; all on by default). For any language
//! without a compiled-in grammar — or when parsing fails — the provider
//! degrades to the exact behavior of
//! [`codemmet_core::context::NullContextProvider`] for `context_at`, and
//! to plain whole-line snapping for `validate_wrap`.
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
    not(any(
        feature = "lang-rust",
        feature = "lang-python",
        feature = "lang-c",
        feature = "lang-cpp",
        feature = "lang-javascript",
        feature = "lang-typescript",
        feature = "lang-go",
    )),
    allow(clippy::missing_const_for_fn)
)]
fn language_for(id: &str) -> Option<tree_sitter::Language> {
    match id {
        #[cfg(feature = "lang-rust")]
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        #[cfg(feature = "lang-python")]
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        #[cfg(feature = "lang-c")]
        "c" => Some(tree_sitter_c::LANGUAGE.into()),
        #[cfg(feature = "lang-cpp")]
        "cpp" => Some(tree_sitter_cpp::LANGUAGE.into()),
        #[cfg(feature = "lang-javascript")]
        "javascript" => Some(tree_sitter_javascript::LANGUAGE.into()),
        #[cfg(feature = "lang-typescript")]
        "typescript" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        #[cfg(feature = "lang-go")]
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
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
        "c" => c_scope(kind),
        "cpp" => cpp_scope(kind),
        "javascript" => js_scope(kind),
        "typescript" => ts_scope(kind),
        "go" => go_scope(kind),
        _ => None,
    }
}

/// C scope table (also the base of the C++ table).
fn c_scope(kind: &str) -> Option<Scope> {
    match kind {
        "function_definition" => Some(Scope::FnBody),
        "for_statement" | "while_statement" | "do_statement" => Some(Scope::Loop),
        "if_statement" => Some(Scope::Conditional),
        "switch_statement" => Some(Scope::MatchBody),
        "struct_specifier" | "enum_specifier" | "union_specifier" => Some(Scope::TypeBody),
        _ => None,
    }
}

/// C++ scope table: everything C has, plus classes/lambdas/try.
/// `namespace_definition` deliberately maps to no scope.
fn cpp_scope(kind: &str) -> Option<Scope> {
    match kind {
        "class_specifier" => Some(Scope::TypeBody),
        "lambda_expression" => Some(Scope::FnBody),
        "try_statement" => Some(Scope::TryBody),
        _ => c_scope(kind),
    }
}

/// JavaScript scope table (also the base of the TypeScript table).
///
/// Both `class_declaration` and its `class_body` map to [`Scope::TypeBody`]
/// (class *expressions* have no `class_declaration` wrapper); the walk in
/// `context_at` collapses the wrapper/body pair to a single scope.
fn js_scope(kind: &str) -> Option<Scope> {
    match kind {
        "function_declaration"
        | "function_expression"
        | "arrow_function"
        | "method_definition"
        | "generator_function_declaration" => Some(Scope::FnBody),
        "class_declaration" | "class_body" => Some(Scope::TypeBody),
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement" => {
            Some(Scope::Loop)
        }
        "if_statement" => Some(Scope::Conditional),
        "switch_statement" => Some(Scope::MatchBody),
        "try_statement" => Some(Scope::TryBody),
        _ => None,
    }
}

/// TypeScript scope table: JavaScript plus interfaces and enums.
fn ts_scope(kind: &str) -> Option<Scope> {
    match kind {
        "interface_declaration" | "enum_declaration" => Some(Scope::TypeBody),
        _ => js_scope(kind),
    }
}

/// Go scope table.
fn go_scope(kind: &str) -> Option<Scope> {
    match kind {
        "function_declaration" | "method_declaration" | "func_literal" => Some(Scope::FnBody),
        "for_statement" => Some(Scope::Loop),
        "if_statement" => Some(Scope::Conditional),
        "expression_switch_statement" | "type_switch_statement" | "select_statement" => {
            Some(Scope::MatchBody)
        }
        "struct_type" | "interface_type" => Some(Scope::TypeBody),
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
        "c" => matches!(kind, "string_literal" | "char_literal"),
        "cpp" => matches!(
            kind,
            "string_literal" | "char_literal" | "raw_string_literal"
        ),
        "javascript" | "typescript" => matches!(kind, "string" | "template_string"),
        "go" => matches!(
            kind,
            "interpreted_string_literal" | "raw_string_literal" | "rune_literal"
        ),
        _ => false,
    }
}

/// Start byte of the body/block child of a scope-bearing node, if present.
///
/// Most constructs across the grammars expose their suite via a `body`
/// field; `if` constructs use `consequence`. As a fallback scan for a
/// block-like named child (Go's `struct_type` has an unnamed
/// `field_declaration_list`), and finally for a bare `{` token (Go's
/// switch/select statements carry their cases directly). `None` means the
/// body is missing entirely (half-typed construct), which callers treat
/// as "already inside".
fn body_start(node: Node<'_>) -> Option<usize> {
    for field in ["body", "consequence"] {
        if let Some(child) = node.child_by_field_name(field) {
            return Some(child.start_byte());
        }
    }
    let mut cursor = node.walk();
    if let Some(child) = node.named_children(&mut cursor).find(|c| {
        matches!(
            c.kind(),
            "block"
                | "declaration_list"
                | "match_block"
                | "class_body"
                | "statement_block"
                | "compound_statement"
                | "field_declaration_list"
                | "enumerator_list"
                | "interface_body"
                | "enum_body"
        )
    }) {
        return Some(child.start_byte());
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|c| c.kind() == "{")
        .map(|c| c.start_byte())
}

/// Node kinds whose named children are statements/items — the containers
/// wrap snapping aligns selections against. The set is shared across
/// languages; the kinds are distinct enough that collisions are harmless.
fn is_container_kind(kind: &str) -> bool {
    matches!(
        kind,
        // Roots: rust/go, python, c/cpp, js/ts.
        "source_file" | "module" | "translation_unit" | "program"
        // Statement blocks: rust/python/go, js/ts, c/cpp.
        | "block" | "statement_block" | "compound_statement"
        // Item/member lists.
        | "declaration_list" | "field_declaration_list" | "class_body"
        | "interface_body" | "enum_body" | "enumerator_list" | "match_block"
        // Switch internals: js/ts, c/cpp, go.
        | "switch_body" | "switch_case" | "switch_default" | "case_statement"
        | "expression_case" | "type_case" | "communication_case" | "default_case"
    )
}

/// One end of a wrap selection, resolved against the tree.
#[derive(Debug, Clone, Copy)]
enum Endpoint<'t> {
    /// The offset sits inside this statement (the outermost node covering
    /// the offset whose parent is a container).
    Statement(Node<'t>),
    /// The offset sits in a container's own whitespace (between
    /// statements); the node is the container itself.
    Floating(Node<'t>),
}

/// Resolve `offset` to the statement (or container gap) holding it.
fn endpoint_at(root: Node<'_>, offset: usize) -> Endpoint<'_> {
    let mut node = root
        .named_descendant_for_byte_range(offset, offset)
        .unwrap_or(root);
    if is_container_kind(node.kind()) {
        return Endpoint::Floating(node);
    }
    while let Some(parent) = node.parent() {
        if is_container_kind(parent.kind()) {
            break;
        }
        node = parent;
    }
    Endpoint::Statement(node)
}

/// Whether `outer` strictly encloses `inner` (distinct nodes, byte-wise).
fn encloses(outer: Node<'_>, inner: Node<'_>) -> bool {
    outer.id() != inner.id()
        && outer.start_byte() <= inner.start_byte()
        && inner.end_byte() <= outer.end_byte()
}

/// The ancestor of `node` that is a *direct* child of `container`.
fn child_of<'t>(container: Node<'t>, node: Node<'t>) -> Option<Node<'t>> {
    let mut node = node;
    loop {
        let parent = node.parent()?;
        if parent.id() == container.id() {
            return Some(node);
        }
        node = parent;
    }
}

/// Whether `a` and `b` are distinct siblings (same parent node).
fn siblings(a: Node<'_>, b: Node<'_>) -> bool {
    match (a.parent(), b.parent()) {
        (Some(pa), Some(pb)) => pa.id() == pb.id(),
        _ => false,
    }
}

/// The rejection every unreconcilable endpoint pair produces.
fn cross_scope() -> WrapSelection {
    WrapSelection::Invalid {
        reason: "selection crosses scope boundaries".to_owned(),
    }
}

/// Extend `[start, end)` outward to whole lines (wrap is linewise).
///
/// The result excludes the trailing newline; an `end` sitting just past a
/// newline stays on that line instead of swallowing the next one.
fn extend_to_lines(text: &str, start: usize, end: usize) -> Range<usize> {
    let start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let mut end = end.max(start);
    if end > start && text.as_bytes().get(end - 1) == Some(&b'\n') {
        end -= 1;
    }
    let end = text[end..].find('\n').map_or(text.len(), |i| end + i);
    start..end
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
        // The node that most recently contributed a scope, for collapsing
        // wrapper/body pairs (see below).
        let mut last_scope_node: Option<(usize, Scope)> = None;
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
                    // A wrapper and its own body node (JS `class_declaration`
                    // containing `class_body`) both map to the same scope;
                    // collapse the pair to one entry. Only a *direct* parent
                    // suppresses the push, so genuinely nested same-kind
                    // scopes (loop in loop) still stack.
                    let wrapper_pushed = node
                        .parent()
                        .is_some_and(|parent| last_scope_node == Some((parent.id(), scope)));
                    if !wrapper_pushed {
                        scopes.push(scope);
                    }
                    last_scope_node = Some((node.id(), scope));
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

    /// Snap a wrap selection to statement boundaries.
    ///
    /// Each endpoint resolves to the statement holding it (or to the gap
    /// between statements). Endpoints reconcile when they land in the same
    /// statement, in sibling statements, or when one statement encloses
    /// the other endpoint entirely; the snapped range then grows outward
    /// to statement starts/ends and finally to whole lines. Endpoints at
    /// different nesting levels with no enclosing statement (the tail of
    /// one function plus the head of the next) are rejected.
    fn validate_wrap(&mut self, doc: &DocSnapshot, range: Range<usize>) -> WrapSelection {
        let text = doc.text.as_str();
        let start = clamp_offset(text, range.start);
        let end = clamp_offset(text, range.end.max(range.start));
        let Some(tree) = self.tree_for(doc) else {
            // No grammar for this language (or the parse failed): fall
            // back to plain whole-line snapping.
            return WrapSelection::Valid {
                range: extend_to_lines(text, start, end),
            };
        };
        if start >= end {
            return WrapSelection::Invalid {
                reason: "empty selection".to_owned(),
            };
        }

        let root = tree.root_node();
        let node_range = |node: Node<'_>| node.start_byte()..node.end_byte();
        // `end` is exclusive, so the last byte inside the selection is at
        // `end - 1` (mid-char bytes are fine for tree queries).
        let snapped = match (endpoint_at(root, start), endpoint_at(root, end - 1)) {
            (Endpoint::Statement(s), Endpoint::Statement(e)) => {
                if s.id() == e.id() || encloses(s, e) {
                    node_range(s)
                } else if encloses(e, s) {
                    node_range(e)
                } else if siblings(s, e) {
                    s.start_byte().min(e.start_byte())..s.end_byte().max(e.end_byte())
                } else {
                    return cross_scope();
                }
            }
            (Endpoint::Statement(s), Endpoint::Floating(gap)) => {
                if encloses(s, gap) {
                    // The end floats inside the statement's own body.
                    node_range(s)
                } else if s.parent().is_some_and(|p| p.id() == gap.id()) {
                    // Statement plus trailing whitespace in its container.
                    s.start_byte()..end
                } else {
                    return cross_scope();
                }
            }
            (Endpoint::Floating(gap), Endpoint::Statement(e)) => {
                if encloses(e, gap) {
                    node_range(e)
                } else if let Some(stmt) = child_of(gap, e) {
                    // Leading whitespace in a container, running into a
                    // statement (possibly entered mid-way): pull in the
                    // whole statement at the container's level.
                    start..stmt.end_byte()
                } else {
                    return cross_scope();
                }
            }
            (Endpoint::Floating(a), Endpoint::Floating(b)) => {
                if a.id() == b.id() {
                    // Whitespace/blank lines only: keep the lines as-is.
                    start..end
                } else if let Some(stmt) = child_of(a, b) {
                    // Start floats in an outer container, end floats in a
                    // nested one: pull in the bridging statement.
                    start..stmt.end_byte()
                } else {
                    return cross_scope();
                }
            }
        };
        WrapSelection::Valid {
            range: extend_to_lines(text, snapped.start, snapped.end),
        }
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

    #[cfg(any(
        feature = "lang-rust",
        feature = "lang-python",
        feature = "lang-c",
        feature = "lang-cpp",
        feature = "lang-javascript",
        feature = "lang-typescript",
        feature = "lang-go",
    ))]
    fn ctx(language: &str, fixture: &str) -> ExpansionContext {
        let (doc, offset) = doc_at(language, fixture);
        TreeSitterProvider::new().context_at(&doc, offset)
    }

    /// Run `validate_wrap` over a fixture with `«` and `»` selection
    /// markers; returns the outcome plus the marker-free document.
    #[cfg(any(feature = "lang-rust", feature = "lang-python"))]
    fn wrap_at(language: &str, fixture: &str) -> (WrapSelection, String) {
        let start = fixture.find('«').expect("fixture contains « marker");
        let text = fixture.replacen('«', "", 1);
        let end = text.find('»').expect("fixture contains » marker");
        let text = text.replacen('»', "", 1);
        let doc = DocSnapshot {
            language: language.into(),
            text,
        };
        let result = TreeSitterProvider::new().validate_wrap(&doc, start..end);
        (result, doc.text)
    }

    /// Assert a wrap fixture is valid and return the snapped text slice.
    #[cfg(any(feature = "lang-rust", feature = "lang-python"))]
    fn wrap_snapped(language: &str, fixture: &str) -> String {
        match wrap_at(language, fixture) {
            (WrapSelection::Valid { range }, text) => text[range].to_owned(),
            (WrapSelection::Invalid { reason }, _) => panic!("unexpected invalid: {reason}"),
        }
    }

    /// Assert a wrap fixture is rejected and return the reason.
    #[cfg(any(feature = "lang-rust", feature = "lang-python"))]
    fn wrap_reason(language: &str, fixture: &str) -> String {
        match wrap_at(language, fixture) {
            (WrapSelection::Invalid { reason }, _) => reason,
            (WrapSelection::Valid { range }, text) => {
                panic!("unexpected valid: {:?}", &text[range])
            }
        }
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
        fn wrap_mid_statement_snaps_to_the_whole_statement() {
            // The selection covers a fragment of a *multi-line* statement;
            // the snap must grow to the statement, not just to the lines.
            let got = wrap_snapped(
                "rust",
                "fn main() {\n    foo(\n        «1,\n        2»,\n    );\n}\n",
            );
            assert_eq!(got, "    foo(\n        1,\n        2,\n    );");
        }

        #[test]
        fn wrap_two_statement_selection_snaps_to_both() {
            let got = wrap_snapped("rust", "fn main() {\n    let a« = 1;\n    let b» = 2;\n}\n");
            assert_eq!(got, "    let a = 1;\n    let b = 2;");
        }

        #[test]
        fn wrap_whole_function_selection_stays() {
            let got = wrap_snapped("rust", "«fn a() {\n    let x = 1;\n}»\nfn b() {}\n");
            assert_eq!(got, "fn a() {\n    let x = 1;\n}");
        }

        #[test]
        fn wrap_signature_into_body_snaps_to_the_whole_function() {
            let got = wrap_snapped("rust", "«fn a() {\n    let x» = 1;\n}\nfn b() {}\n");
            assert_eq!(got, "fn a() {\n    let x = 1;\n}");
        }

        #[test]
        fn wrap_cross_function_selection_is_invalid() {
            let reason = wrap_reason(
                "rust",
                "fn a() {\n    let «x = 1;\n}\nfn b»() {\n    let y = 2;\n}\n",
            );
            assert_eq!(reason, "selection crosses scope boundaries");
        }

        #[test]
        fn wrap_body_tail_into_trailing_gap_is_invalid() {
            // Tail of `a`'s body plus the blank line *after* the function:
            // the region would cross the closing brace.
            let reason = wrap_reason("rust", "fn a() {\n    let «x = 1;\n}\n»\nfn b() {}\n");
            assert_eq!(reason, "selection crosses scope boundaries");
        }

        #[test]
        fn wrap_empty_selection_is_invalid() {
            let (result, _) = wrap_at("rust", "fn main() {«»}\n");
            match result {
                WrapSelection::Invalid { reason } => assert_eq!(reason, "empty selection"),
                WrapSelection::Valid { range } => panic!("unexpected valid: {range:?}"),
            }
        }

        #[test]
        fn wrap_whitespace_selection_keeps_the_lines() {
            let got = wrap_snapped(
                "rust",
                "fn main() {\n    let a = 1;\n« » \n    let b = 2;\n}\n",
            );
            assert_eq!(got, "  ");
        }

        #[test]
        fn wrap_comment_selection_snaps_to_the_comment() {
            let got = wrap_snapped("rust", "fn main() {\n    // he«llo wor»ld\n}\n");
            assert_eq!(got, "    // hello world");
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

        #[test]
        fn wrap_mid_statement_snaps_to_the_whole_statement() {
            let got = wrap_snapped("python", "def a():\n    foo(«1,\n        2»)\n");
            assert_eq!(got, "    foo(1,\n        2)");
        }

        #[test]
        fn wrap_two_statement_selection_snaps_to_both() {
            let got = wrap_snapped("python", "def a():\n    x «= 1\n    y »= 2\n    z = 3\n");
            assert_eq!(got, "    x = 1\n    y = 2");
        }

        #[test]
        fn wrap_cross_def_selection_is_invalid() {
            let reason = wrap_reason(
                "python",
                "def a():\n    x = 1\n    «y = 2\ndef b»():\n    z = 3\n",
            );
            assert_eq!(reason, "selection crosses scope boundaries");
        }
    }

    #[cfg(feature = "lang-c")]
    mod c {
        use super::*;

        #[test]
        fn function_body_is_fn_body() {
            let c = ctx("c", "int main(void) {\n    §\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody]);
        }

        #[test]
        fn struct_body_is_type_body() {
            let c = ctx("c", "struct point {\n    int §\n};\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody]);
        }

        #[test]
        fn for_body_inside_fn_stacks_scopes() {
            let c = ctx(
                "c",
                "int main(void) {\n    for (int i = 0; i < 3; i++) {\n        §\n    }\n}\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody, Scope::Loop]);
        }

        #[test]
        fn inside_string_literal() {
            let c = ctx("c", "int main(void) { char *s = \"he§llo\"; }\n");
            assert!(c.in_string_or_comment);
        }
    }

    #[cfg(feature = "lang-cpp")]
    mod cpp {
        use super::*;

        #[test]
        fn method_in_class_stacks_scopes() {
            let c = ctx(
                "cpp",
                "class P {\n  public:\n    void f() {\n        §\n    }\n};\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody, Scope::FnBody]);
        }

        #[test]
        fn lambda_body_is_fn_body() {
            let c = ctx(
                "cpp",
                "int main() {\n    auto f = []() {\n        §\n    };\n}\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody, Scope::FnBody]);
        }

        #[test]
        fn try_body_is_try() {
            let c = ctx(
                "cpp",
                "int main() {\n    try {\n        §\n    } catch (int e) {\n    }\n}\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody, Scope::TryBody]);
        }

        #[test]
        fn namespace_adds_no_scope() {
            let c = ctx("cpp", "namespace n {\nint §x;\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root]);
        }
    }

    #[cfg(feature = "lang-javascript")]
    mod javascript {
        use super::*;

        #[test]
        fn class_body_yields_a_single_type_body() {
            // `class_declaration` and `class_body` both map to TypeBody;
            // the wrapper/body pair must collapse to one scope.
            let c = ctx("javascript", "class A {\n  x = 1;\n  §\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody]);
        }

        #[test]
        fn method_in_class_stacks_scopes() {
            let c = ctx("javascript", "class A {\n  m() {\n    §\n  }\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody, Scope::FnBody]);
        }

        #[test]
        fn arrow_body_is_fn_body() {
            let c = ctx("javascript", "const f = () => {\n  §\n};\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody]);
        }

        #[test]
        fn inside_template_string() {
            let c = ctx("javascript", "const s = `he§llo`;\n");
            assert!(c.in_string_or_comment);
        }
    }

    #[cfg(feature = "lang-typescript")]
    mod typescript {
        use super::*;

        #[test]
        fn interface_body_is_type_body() {
            let c = ctx("typescript", "interface A {\n  x: number;\n  §\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody]);
        }

        #[test]
        fn enum_body_is_type_body() {
            let c = ctx("typescript", "enum E {\n  A,\n  §\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody]);
        }

        #[test]
        fn function_body_is_fn_body() {
            let c = ctx("typescript", "function f(): void {\n  §\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody]);
        }
    }

    #[cfg(feature = "lang-go")]
    mod go {
        use super::*;

        #[test]
        fn func_body_is_fn_body() {
            let c = ctx("go", "package main\n\nfunc main() {\n\tx := 1\n\t§\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody]);
            assert_eq!(c.indent_unit, IndentUnit::Tab);
        }

        #[test]
        fn struct_body_is_type_body() {
            let c = ctx("go", "package main\n\ntype Foo struct {\n\t§\n}\n");
            assert_eq!(c.scopes, vec![Scope::Root, Scope::TypeBody]);
        }

        #[test]
        fn switch_case_is_match_body() {
            let c = ctx(
                "go",
                "package main\n\nfunc f(x int) {\n\tswitch x {\n\tcase 1:\n\t\t§\n\t}\n}\n",
            );
            assert_eq!(c.scopes, vec![Scope::Root, Scope::FnBody, Scope::MatchBody]);
        }

        #[test]
        fn inside_raw_string() {
            let c = ctx("go", "package main\n\nvar s = `he§llo`\n");
            assert!(c.in_string_or_comment);
        }
    }

    #[test]
    fn unsupported_language_matches_null_provider() {
        let (doc, offset) = doc_at("cobol", "func main() {\n\t§x\n}\n");
        let ts = TreeSitterProvider::new().context_at(&doc, offset);
        let null = NullContextProvider.context_at(&doc, offset);
        assert_eq!(ts.scopes, null.scopes);
        assert_eq!(ts.raw_node_kinds, null.raw_node_kinds);
        assert_eq!(ts.in_string_or_comment, null.in_string_or_comment);
        assert_eq!(ts.base_indent, null.base_indent);
        assert_eq!(ts.indent_unit, null.indent_unit);
    }

    #[test]
    fn wrap_line_snaps_unsupported_languages() {
        let doc = DocSnapshot {
            language: "cobol".into(),
            text: "func main() {\n\tx := 1\n}\n".into(),
        };
        // Selection sits mid-`x := 1`; snaps to that whole line.
        match TreeSitterProvider::new().validate_wrap(&doc, 16..19) {
            WrapSelection::Valid { range } => assert_eq!(range, 14..21),
            WrapSelection::Invalid { reason } => panic!("unexpected invalid: {reason}"),
        }
    }
}
