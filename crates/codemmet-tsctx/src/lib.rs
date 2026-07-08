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
    DocSnapshot, ExpansionContext, FillTarget, ImportScan, IndentUnit, NullContextProvider, Scope,
    WrapSelection,
};
use codemmet_core::lang::profile;
use tree_sitter::{Node, Tree};

pub use codemmet_core::context::ContextProvider;

mod codegen;
mod extract;
mod imports;

/// Whether this build carries a grammar for `language_id`.
///
/// `language_id` values are canonical profile ids (`"rust"`, `"python"`).
pub fn supports(language_id: &str) -> bool {
    spec_for(language_id).is_some()
}

/// Field names that hold a scope-bearing node's body/suite. Most
/// constructs expose their suite via `body`; `if` constructs use
/// `consequence`. Shared verbatim across every language today.
#[cfg(any(
    feature = "lang-rust",
    feature = "lang-python",
    feature = "lang-c",
    feature = "lang-cpp",
    feature = "lang-javascript",
    feature = "lang-typescript",
    feature = "lang-go",
))]
const BODY_FIELDS: &[&str] = &["body", "consequence"];

/// Block-like named child kinds scanned as a body fallback when neither
/// `body` nor `consequence` is present. Shared across languages.
#[cfg(any(
    feature = "lang-rust",
    feature = "lang-python",
    feature = "lang-c",
    feature = "lang-cpp",
    feature = "lang-javascript",
    feature = "lang-typescript",
    feature = "lang-go",
))]
const BLOCK_KINDS: &[&str] = &[
    "block",
    "declaration_list",
    "match_block",
    "class_body",
    "statement_block",
    "compound_statement",
    "field_declaration_list",
    "enumerator_list",
    "interface_body",
    "enum_body",
];

/// Node kinds whose named children are statements/items — the containers
/// wrap snapping aligns selections against. Shared across languages; the
/// kinds are distinct enough that cross-language collisions are harmless.
#[cfg(any(
    feature = "lang-rust",
    feature = "lang-python",
    feature = "lang-c",
    feature = "lang-cpp",
    feature = "lang-javascript",
    feature = "lang-typescript",
    feature = "lang-go",
))]
const CONTAINER_KINDS: &[&str] = &[
    // Roots: rust/go, python, c/cpp, js/ts.
    "source_file",
    "module",
    "translation_unit",
    "program",
    // Statement blocks: rust/python/go, js/ts, c/cpp.
    "block",
    "statement_block",
    "compound_statement",
    // Item/member lists.
    "declaration_list",
    "field_declaration_list",
    "class_body",
    "interface_body",
    "enum_body",
    "enumerator_list",
    "match_block",
    // Switch internals: js/ts, c/cpp, go.
    "switch_body",
    "switch_case",
    "switch_default",
    "case_statement",
    "expression_case",
    "type_case",
    "communication_case",
    "default_case",
];

/// All per-language knowledge in one place: the grammar plus the kind
/// tables the tree walk consults.
///
/// Adding a language means adding one `LangSpec` and one arm in
/// [`spec_for`] — there are no scattered `match language` sites left to
/// keep in sync, so it is no longer possible to add a language's scopes
/// while silently forgetting its string kinds.
struct LangSpec {
    /// Constructs the compiled-in tree-sitter grammar for this language.
    grammar: fn() -> tree_sitter::Language,
    /// Raw node kind → normalized [`Scope`]. Kinds absent from the table
    /// contribute only to `raw_node_kinds`; [`Scope::Unknown`] is never
    /// produced.
    scopes: &'static [(&'static str, Scope)],
    /// String-literal node kinds (comments are matched by kind substring
    /// instead, which also covers doc comments).
    string_kinds: &'static [&'static str],
    /// Field names holding a scope-bearing node's body/suite.
    body_fields: &'static [&'static str],
    /// Block-like named child kinds used as a body fallback.
    block_kinds: &'static [&'static str],
    /// Node kinds whose named children are statements/items.
    container_kinds: &'static [&'static str],
}

impl LangSpec {
    /// Map a raw node kind to its normalized [`Scope`], if any.
    fn scope_for(&self, kind: &str) -> Option<Scope> {
        self.scopes
            .iter()
            .find(|&&(k, _)| k == kind)
            .map(|&(_, scope)| scope)
    }

    /// Whether `kind` is one of this language's string-literal kinds.
    fn is_string_kind(&self, kind: &str) -> bool {
        self.string_kinds.contains(&kind)
    }

    /// Whether `kind` is a container (its named children are statements).
    fn is_container_kind(&self, kind: &str) -> bool {
        self.container_kinds.contains(&kind)
    }

    /// Start byte of the body/block child of a scope-bearing node, if
    /// present.
    ///
    /// Checks the body fields first, then scans for a block-like named
    /// child (Go's `struct_type` has an unnamed `field_declaration_list`),
    /// and finally for a bare `{` token (Go's switch/select statements
    /// carry their cases directly). `None` means the body is missing
    /// entirely (half-typed construct), which callers treat as "already
    /// inside".
    fn body_start(&self, node: Node<'_>) -> Option<usize> {
        for &field in self.body_fields {
            if let Some(child) = node.child_by_field_name(field) {
                return Some(child.start_byte());
            }
        }
        let mut cursor = node.walk();
        if let Some(child) = node
            .named_children(&mut cursor)
            .find(|c| self.block_kinds.contains(&c.kind()))
        {
            return Some(child.start_byte());
        }
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .find(|c| c.kind() == "{")
            .map(|c| c.start_byte())
    }

    /// The body/block *node* of a scope-bearing node (the class body,
    /// declaration list, suite, …), for structural injection.
    fn body_node<'t>(&self, node: Node<'t>) -> Option<Node<'t>> {
        for &field in self.body_fields {
            if let Some(child) = node.child_by_field_name(field) {
                return Some(child);
            }
        }
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .find(|c| self.block_kinds.contains(&c.kind()))
    }
}

/// Per-language spec for `language`, or `None` when this build carries no
/// grammar for it. The `#[cfg(feature = "lang-…")]` arms live here and
/// nowhere else.
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
fn spec_for(language: &str) -> Option<&'static LangSpec> {
    match language {
        #[cfg(feature = "lang-rust")]
        "rust" => Some(&RUST),
        #[cfg(feature = "lang-python")]
        "python" => Some(&PYTHON),
        #[cfg(feature = "lang-c")]
        "c" => Some(&C),
        #[cfg(feature = "lang-cpp")]
        "cpp" => Some(&CPP),
        #[cfg(feature = "lang-javascript")]
        "javascript" => Some(&JAVASCRIPT),
        #[cfg(feature = "lang-typescript")]
        "typescript" => Some(&TYPESCRIPT),
        #[cfg(feature = "lang-go")]
        "go" => Some(&GO),
        _ => None,
    }
}

/// A structural insertion: splice [`Self::text`] at byte [`Self::offset`] in an
/// existing file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Injection {
    /// Byte offset to insert at (a token boundary, so a valid char boundary).
    pub offset: usize,
    /// The text to insert, already indented and surrounded by the newlines that
    /// place it as a new member.
    pub text: String,
}

/// Find a structural insertion point for `content` in an existing file.
///
/// It injects inside the body of a type-like container (class, struct, impl,
/// trait, interface, …), so `content` becomes a new member at the body's
/// indentation. Returns `None` when the language is unsupported or the file has
/// no such container — the caller should fall back to appending.
///
/// `into` names the container to target (matched against its name/type); when
/// `None`, or no container has that name, the **last** container is used. For a
/// C/C++ class with a `public:` section the member lands at the end of that
/// section rather than at the very end of the body (which might be `private`).
#[must_use]
pub fn inject(
    language_id: &str,
    text: &str,
    content: &str,
    indent_unit: &str,
    into: Option<&str>,
) -> Option<Injection> {
    let spec = spec_for(language_id)?;
    let language = (spec.grammar)();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok()?;
    let tree = parser.parse(text, None)?;

    let container = choose_container(tree.root_node(), spec, text, into)?;
    let body = spec.body_node(container)?;
    let member_indent = format!("{}{indent_unit}", line_indent(text, container.start_byte()));
    let block = indent_block(content.trim_end_matches('\n'), &member_indent);

    // Prefer the end of a C/C++ `public:` section, so a declared method lands
    // among the public members rather than after a trailing `private:` one.
    if let Some(offset) = public_section_end(body, text) {
        return Some(Injection {
            offset,
            text: format!("\n{block}"),
        });
    }

    let mut cursor = body.walk();
    let members: Vec<Node<'_>> = body
        .named_children(&mut cursor)
        .filter(|n| n.kind() != "comment")
        .collect();
    Some(members.last().map_or_else(
        || {
            // Empty body: insert just after the opening brace, then re-close.
            let mut cursor = body.walk();
            let open = body
                .children(&mut cursor)
                .find(|c| c.kind() == "{")
                .map_or_else(|| body.start_byte(), |b| b.end_byte());
            let close_indent = line_indent(text, container.start_byte());
            Injection {
                offset: open,
                text: format!("\n{block}\n{close_indent}"),
            }
        },
        // Insert after the last member, keeping the body's closing on its line.
        |last| Injection {
            offset: last.end_byte(),
            text: format!("\n{block}"),
        },
    ))
}

/// Choose the container to inject into: the one named `into` (matched against
/// its `name`/`type` field), else the last outermost type-body in document
/// order. Outermost: the walk does not descend into a container once found, so
/// a nested type never shadows the type you meant.
fn choose_container<'t>(
    root: Node<'t>,
    spec: &LangSpec,
    text: &str,
    into: Option<&str>,
) -> Option<Node<'t>> {
    let mut containers: Vec<Node<'t>> = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if spec.scope_for(node.kind()) == Some(Scope::TypeBody) {
            containers.push(node);
            continue;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    containers.sort_by_key(Node::start_byte);
    if let Some(name) = into
        && let Some(hit) = containers
            .iter()
            .rev()
            .find(|c| container_name(**c, text) == Some(name))
    {
        return Some(*hit);
    }
    containers.last().copied()
}

/// A container's declared name (`name` field, else `type` for a Rust `impl`).
fn container_name<'a>(node: Node<'_>, text: &'a str) -> Option<&'a str> {
    node.child_by_field_name("name")
        .or_else(|| node.child_by_field_name("type"))
        .map(|n| &text[n.byte_range()])
}

/// The byte offset ending the last `public:` section of a C/C++ body (after its
/// last member, or after the `public:` specifier itself when the section is
/// empty), or `None` when the body has no `public:` specifier.
fn public_section_end(body: Node<'_>, text: &str) -> Option<usize> {
    let mut cursor = body.walk();
    let mut in_public = false;
    let mut seen_public = false;
    let mut anchor = None;
    for child in body.children(&mut cursor) {
        if child.kind() == "access_specifier" {
            in_public = text[child.byte_range()].trim_start().starts_with("public");
            if in_public {
                seen_public = true;
                anchor = Some(child.end_byte());
            }
        } else if in_public && child.is_named() && child.kind() != "comment" {
            anchor = Some(child.end_byte());
        }
    }
    seen_public.then_some(anchor).flatten()
}

/// The leading whitespace of the line containing `offset`.
fn line_indent(text: &str, offset: usize) -> String {
    let offset = offset.min(text.len());
    let start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    text[start..offset]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

/// Prefix every non-empty line of `content` with `indent`.
fn indent_block(content: &str, indent: &str) -> String {
    content
        .split('\n')
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(feature = "lang-rust")]
static RUST: LangSpec = LangSpec {
    grammar: || tree_sitter_rust::LANGUAGE.into(),
    scopes: &[
        ("impl_item", Scope::TypeBody),
        ("trait_item", Scope::TypeBody),
        ("function_item", Scope::FnBody),
        ("closure_expression", Scope::FnBody),
        ("for_expression", Scope::Loop),
        ("while_expression", Scope::Loop),
        ("loop_expression", Scope::Loop),
        ("if_expression", Scope::Conditional),
        ("match_expression", Scope::MatchBody),
    ],
    string_kinds: &["string_literal", "raw_string_literal", "char_literal"],
    body_fields: BODY_FIELDS,
    block_kinds: BLOCK_KINDS,
    container_kinds: CONTAINER_KINDS,
};

#[cfg(feature = "lang-python")]
static PYTHON: LangSpec = LangSpec {
    grammar: || tree_sitter_python::LANGUAGE.into(),
    scopes: &[
        ("class_definition", Scope::TypeBody),
        ("function_definition", Scope::FnBody),
        ("for_statement", Scope::Loop),
        ("while_statement", Scope::Loop),
        ("if_statement", Scope::Conditional),
        ("match_statement", Scope::MatchBody),
        ("try_statement", Scope::TryBody),
    ],
    string_kinds: &["string", "string_content", "concatenated_string"],
    body_fields: BODY_FIELDS,
    block_kinds: BLOCK_KINDS,
    container_kinds: CONTAINER_KINDS,
};

/// C scopes are also the base of the C++ table below.
#[cfg(feature = "lang-c")]
static C: LangSpec = LangSpec {
    grammar: || tree_sitter_c::LANGUAGE.into(),
    scopes: &[
        ("function_definition", Scope::FnBody),
        ("for_statement", Scope::Loop),
        ("while_statement", Scope::Loop),
        ("do_statement", Scope::Loop),
        ("if_statement", Scope::Conditional),
        ("switch_statement", Scope::MatchBody),
        ("struct_specifier", Scope::TypeBody),
        ("enum_specifier", Scope::TypeBody),
        ("union_specifier", Scope::TypeBody),
    ],
    string_kinds: &["string_literal", "char_literal"],
    body_fields: BODY_FIELDS,
    block_kinds: BLOCK_KINDS,
    container_kinds: CONTAINER_KINDS,
};

/// C++ = every C scope, plus classes/lambdas/try. `namespace_definition`
/// deliberately maps to no scope. The C kinds are spelled out here rather
/// than delegated so the whole table reads in one place.
#[cfg(feature = "lang-cpp")]
static CPP: LangSpec = LangSpec {
    grammar: || tree_sitter_cpp::LANGUAGE.into(),
    scopes: &[
        // C++-specific.
        ("class_specifier", Scope::TypeBody),
        ("lambda_expression", Scope::FnBody),
        ("try_statement", Scope::TryBody),
        // Inherited from C.
        ("function_definition", Scope::FnBody),
        ("for_statement", Scope::Loop),
        ("while_statement", Scope::Loop),
        ("do_statement", Scope::Loop),
        ("if_statement", Scope::Conditional),
        ("switch_statement", Scope::MatchBody),
        ("struct_specifier", Scope::TypeBody),
        ("enum_specifier", Scope::TypeBody),
        ("union_specifier", Scope::TypeBody),
    ],
    string_kinds: &["string_literal", "char_literal", "raw_string_literal"],
    body_fields: BODY_FIELDS,
    block_kinds: BLOCK_KINDS,
    container_kinds: CONTAINER_KINDS,
};

/// JavaScript scopes are also the base of the TypeScript table below.
///
/// Both `class_declaration` and its `class_body` map to [`Scope::TypeBody`]
/// (class *expressions* have no `class_declaration` wrapper); the walk in
/// `context_at` collapses the wrapper/body pair to a single scope.
#[cfg(feature = "lang-javascript")]
static JAVASCRIPT: LangSpec = LangSpec {
    grammar: || tree_sitter_javascript::LANGUAGE.into(),
    scopes: &[
        ("function_declaration", Scope::FnBody),
        ("function_expression", Scope::FnBody),
        ("arrow_function", Scope::FnBody),
        ("method_definition", Scope::FnBody),
        ("generator_function_declaration", Scope::FnBody),
        ("class_declaration", Scope::TypeBody),
        ("class_body", Scope::TypeBody),
        ("for_statement", Scope::Loop),
        ("for_in_statement", Scope::Loop),
        ("while_statement", Scope::Loop),
        ("do_statement", Scope::Loop),
        ("if_statement", Scope::Conditional),
        ("switch_statement", Scope::MatchBody),
        ("try_statement", Scope::TryBody),
    ],
    string_kinds: &["string", "template_string"],
    body_fields: BODY_FIELDS,
    block_kinds: BLOCK_KINDS,
    container_kinds: CONTAINER_KINDS,
};

/// TypeScript = every JavaScript scope, plus interfaces and enums. The
/// JavaScript kinds are spelled out here rather than delegated so the
/// whole table reads in one place.
#[cfg(feature = "lang-typescript")]
static TYPESCRIPT: LangSpec = LangSpec {
    grammar: || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    scopes: &[
        // TypeScript-specific.
        ("interface_declaration", Scope::TypeBody),
        ("enum_declaration", Scope::TypeBody),
        // Inherited from JavaScript.
        ("function_declaration", Scope::FnBody),
        ("function_expression", Scope::FnBody),
        ("arrow_function", Scope::FnBody),
        ("method_definition", Scope::FnBody),
        ("generator_function_declaration", Scope::FnBody),
        ("class_declaration", Scope::TypeBody),
        ("class_body", Scope::TypeBody),
        ("for_statement", Scope::Loop),
        ("for_in_statement", Scope::Loop),
        ("while_statement", Scope::Loop),
        ("do_statement", Scope::Loop),
        ("if_statement", Scope::Conditional),
        ("switch_statement", Scope::MatchBody),
        ("try_statement", Scope::TryBody),
    ],
    string_kinds: &["string", "template_string"],
    body_fields: BODY_FIELDS,
    block_kinds: BLOCK_KINDS,
    container_kinds: CONTAINER_KINDS,
};

#[cfg(feature = "lang-go")]
static GO: LangSpec = LangSpec {
    grammar: || tree_sitter_go::LANGUAGE.into(),
    scopes: &[
        ("function_declaration", Scope::FnBody),
        ("method_declaration", Scope::FnBody),
        ("func_literal", Scope::FnBody),
        ("for_statement", Scope::Loop),
        ("if_statement", Scope::Conditional),
        ("expression_switch_statement", Scope::MatchBody),
        ("type_switch_statement", Scope::MatchBody),
        ("select_statement", Scope::MatchBody),
        ("struct_type", Scope::TypeBody),
        ("interface_type", Scope::TypeBody),
    ],
    string_kinds: &[
        "interpreted_string_literal",
        "raw_string_literal",
        "rune_literal",
    ],
    body_fields: BODY_FIELDS,
    block_kinds: BLOCK_KINDS,
    container_kinds: CONTAINER_KINDS,
};

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
fn endpoint_at<'t>(spec: &LangSpec, root: Node<'t>, offset: usize) -> Endpoint<'t> {
    let mut node = root
        .named_descendant_for_byte_range(offset, offset)
        .unwrap_or(root);
    if spec.is_container_kind(node.kind()) {
        return Endpoint::Floating(node);
    }
    while let Some(parent) = node.parent() {
        if spec.is_container_kind(parent.kind()) {
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

/// Detect the document's indent unit by scanning *every* line, not just
/// the first indented one; `None` when the document has no indented lines
/// yet (leaving [`indent_unit_for`]'s profile fallback intact).
///
/// A leading tab wins: any line whose leading whitespace begins with a tab
/// makes the whole file [`IndentUnit::Tab`]. Otherwise the unit is the
/// SMALLEST positive space-run width among lines indented with spaces only
/// (one indent level is the shallowest indent seen), so a deeper
/// continuation line no longer inflates the result. Lines that mix a tab
/// after leading spaces are ignored for width (their tab does not begin
/// the run, so it cannot be trusted as a level).
fn detect_indent_unit(text: &str) -> Option<IndentUnit> {
    let mut smallest: Option<usize> = None;
    for line in text.lines() {
        let content = line.trim_start_matches([' ', '\t']);
        if content.is_empty() || content.len() == line.len() {
            continue;
        }
        let ws = &line[..line.len() - content.len()];
        if ws.starts_with('\t') {
            return Some(IndentUnit::Tab);
        }
        if ws.contains('\t') {
            // Leading spaces followed by a tab: width is untrustworthy.
            continue;
        }
        smallest = Some(smallest.map_or(ws.len(), |n| n.min(ws.len())));
    }
    smallest.map(|n| IndentUnit::Spaces(u8::try_from(n).unwrap_or(u8::MAX)))
}

/// Indent unit for `doc`: detected from the text when it has indented
/// lines, else the language profile's default, else `Spaces(4)` for
/// unknown languages.
fn indent_unit_for(doc: &DocSnapshot) -> IndentUnit {
    detect_indent_unit(&doc.text)
        .or_else(|| profile::find(&doc.language).map(|p| p.default_indent.clone()))
        .unwrap_or(IndentUnit::Spaces(4))
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

    /// Parse `doc` (or reuse the cached tree when language + text match)
    /// using `spec`'s grammar.
    ///
    /// `None` on parser failure. The language is already known supported
    /// because the caller resolved `spec` via [`spec_for`].
    fn tree_for(&mut self, doc: &DocSnapshot, spec: &LangSpec) -> Option<Tree> {
        let language = (spec.grammar)();
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
        let Some(spec) = spec_for(&doc.language) else {
            return NullContextProvider.context_at(doc, offset);
        };
        let Some(tree) = self.tree_for(doc, spec) else {
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
            if offset > node.start_byte() && (kind.contains("comment") || spec.is_string_kind(kind))
            {
                in_string_or_comment = true;
            }
            if let Some(scope) = spec.scope_for(kind) {
                // The scope only counts once the cursor is inside the
                // construct's body, not its header/signature. A missing
                // body (broken parse mid-typing) counts anyway.
                if spec.body_start(node).is_none_or(|body| offset >= body) {
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

        let indent_unit = indent_unit_for(doc);
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
            // Buffer-sourced collections for data-driven repeats: enum
            // variants and struct fields (file-wide), plus list bindings in
            // scope at the cursor.
            collections: extract::collections(&doc.language, root, text, offset),
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
        // No grammar for this language (or the parse failed): fall back to
        // plain whole-line snapping.
        let line_snap = || WrapSelection::Valid {
            range: extend_to_lines(text, start, end),
        };
        let Some(spec) = spec_for(&doc.language) else {
            return line_snap();
        };
        let Some(tree) = self.tree_for(doc, spec) else {
            return line_snap();
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
        let snapped = match (
            endpoint_at(spec, root, start),
            endpoint_at(spec, root, end - 1),
        ) {
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

    fn fill_targets(&mut self, doc: &DocSnapshot, offset: usize) -> Vec<FillTarget> {
        let Some(spec) = spec_for(&doc.language) else {
            return Vec::new();
        };
        let Some(tree) = self.tree_for(doc, spec) else {
            return Vec::new();
        };
        let text = doc.text.as_str();
        let offset = clamp_offset(text, offset);
        let indent = indent_unit_for(doc);
        // Only Rust has an enum→match-arm fill today; other languages return
        // nothing until their codegen shapes are added.
        match doc.language.as_str() {
            "rust" => codegen::match_arm_fills(tree.root_node(), text, offset, &indent.as_str()),
            _ => Vec::new(),
        }
    }

    fn import_scan(&mut self, doc: &DocSnapshot) -> Option<ImportScan> {
        let spec = spec_for(&doc.language)?;
        let tree = self.tree_for(doc, spec)?;
        imports::scan(&doc.language, tree.root_node(), &doc.text)
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

    /// Apply [`inject`] to `text` (four-space indent, into `target`) and return
    /// the result, or `None` when no container is found.
    #[cfg(any(feature = "lang-rust", feature = "lang-cpp", feature = "lang-python"))]
    fn injected_into(
        language: &str,
        text: &str,
        content: &str,
        target: Option<&str>,
    ) -> Option<String> {
        inject(language, text, content, "    ", target).map(|inj| {
            let mut out = text.to_owned();
            out.insert_str(inj.offset, &inj.text);
            out
        })
    }

    #[cfg(any(feature = "lang-rust", feature = "lang-cpp", feature = "lang-python"))]
    fn injected(language: &str, text: &str, content: &str) -> Option<String> {
        injected_into(language, text, content, None)
    }

    #[cfg(feature = "lang-cpp")]
    #[test]
    fn inject_adds_a_method_after_the_last_class_member() {
        let src = "class Point {\npublic:\n    double area() const;\n};\n";
        assert_eq!(
            injected("cpp", src, "void scale(double f);").as_deref(),
            Some(
                "class Point {\npublic:\n    double area() const;\n    void scale(double f);\n};\n"
            )
        );
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn inject_adds_a_fn_to_a_rust_impl() {
        let src = "impl Point {\n    fn area(&self) {}\n}\n";
        assert_eq!(
            injected("rust", src, "fn scale(&self) {}").as_deref(),
            Some("impl Point {\n    fn area(&self) {}\n    fn scale(&self) {}\n}\n")
        );
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn inject_adds_a_method_to_a_python_class_indenting_each_line() {
        let src = "class Point:\n    def area(self):\n        pass\n";
        assert_eq!(
            injected("python", src, "def scale(self):\n    pass").as_deref(),
            Some(
                "class Point:\n    def area(self):\n        pass\n    \
                 def scale(self):\n        pass\n"
            )
        );
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn inject_returns_none_without_a_container() {
        assert_eq!(injected("rust", "fn free() {}\n", "fn x() {}"), None);
    }

    #[cfg(feature = "lang-cpp")]
    #[test]
    fn inject_targets_the_public_section_not_the_end() {
        // The member should land at the end of `public:`, ahead of `private:`.
        let src = "class C {\npublic:\n    int a();\nprivate:\n    int b_;\n};\n";
        assert_eq!(
            injected("cpp", src, "int c();").as_deref(),
            Some("class C {\npublic:\n    int a();\n    int c();\nprivate:\n    int b_;\n};\n")
        );
    }

    #[cfg(feature = "lang-cpp")]
    #[test]
    fn inject_into_names_the_target_class() {
        let src = "class A {\npublic:\n    int a();\n};\nclass B {\npublic:\n    int b();\n};\n";
        // Default (no name) takes the last class, `B`.
        assert_eq!(
            injected("cpp", src, "int z();")
                .as_deref()
                .map(|s| s.contains("int b();\n    int z();")),
            Some(true)
        );
        // Naming `A` targets the first class instead.
        let into_a = injected_into("cpp", src, "int z();", Some("A")).expect("A resolves");
        assert!(into_a.contains("int a();\n    int z();"), "{into_a}");
        assert!(into_a.contains("int b();\n};"), "B untouched: {into_a}");
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

        #[test]
        fn space_indented_doc_beats_profile_default() {
            // The Go profile says Tab, but a document already indented with
            // four spaces keeps its own detected unit.
            let c = ctx(
                "go",
                "package main\n\nfunc main() {\n    x := 1\n    §\n}\n",
            );
            assert_eq!(c.indent_unit, IndentUnit::Spaces(4));
        }
    }

    /// Indent unit reported for a fixture with a `§` cursor marker.
    fn indent_unit_at(language: &str, fixture: &str) -> IndentUnit {
        let (doc, offset) = doc_at(language, fixture);
        TreeSitterProvider::new()
            .context_at(&doc, offset)
            .indent_unit
    }

    #[test]
    fn detect_indent_unit_uses_smallest_level_not_first_line() {
        // A deep continuation line (8) precedes the real indent level (4);
        // the smallest positive run wins, so the unit is Spaces(4).
        assert_eq!(
            detect_indent_unit("fn f() {\n        cont\n    real\n}\n"),
            Some(IndentUnit::Spaces(4))
        );
    }

    #[test]
    fn detect_indent_unit_tab_wins_over_later_spaces() {
        // First indented line is a tab; a later space-indented line must
        // not override the tab verdict.
        assert_eq!(
            detect_indent_unit("\ttabbed\n    spaced\n"),
            Some(IndentUnit::Tab)
        );
    }

    #[test]
    fn detect_indent_unit_ignores_space_then_tab_width() {
        // The space-then-tab line's width is untrustworthy and skipped;
        // the clean two-space line decides the unit.
        assert_eq!(
            detect_indent_unit("  clean\n  \tmixed\n"),
            Some(IndentUnit::Spaces(2))
        );
    }

    #[test]
    fn detect_indent_unit_none_when_flat() {
        assert_eq!(detect_indent_unit("a\nb\nc\n"), None);
    }

    #[test]
    fn flat_go_doc_falls_back_to_profile_tab() {
        assert_eq!(indent_unit_at("go", "package main\n§\n"), IndentUnit::Tab);
    }

    #[test]
    fn flat_javascript_doc_falls_back_to_profile_two_spaces() {
        assert_eq!(
            indent_unit_at("javascript", "const x = 1;\n§\n"),
            IndentUnit::Spaces(2)
        );
    }

    #[test]
    fn flat_rust_doc_falls_back_to_profile_four_spaces() {
        assert_eq!(
            indent_unit_at("rust", "fn main() {}\n§\n"),
            IndentUnit::Spaces(4)
        );
    }

    #[test]
    fn flat_unknown_language_doc_falls_back_to_four_spaces() {
        assert_eq!(indent_unit_at("cobol", "x = 1\n§\n"), IndentUnit::Spaces(4));
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

    // ---- Data-driven collection extraction (CX1–CX3) -------------------

    /// The collections `context_at` extracts at the fixture's `§` cursor.
    #[cfg(any(
        feature = "lang-rust",
        feature = "lang-python",
        feature = "lang-c",
        feature = "lang-cpp",
        feature = "lang-javascript",
        feature = "lang-typescript",
        feature = "lang-go",
    ))]
    fn cols(
        language: &str,
        fixture: &str,
    ) -> std::collections::BTreeMap<String, codemmet_core::context::Collection> {
        ctx(language, fixture).collections
    }

    /// The `value`s of a named collection, in order (panics if absent).
    #[cfg(any(
        feature = "lang-rust",
        feature = "lang-python",
        feature = "lang-c",
        feature = "lang-cpp",
        feature = "lang-javascript",
        feature = "lang-typescript",
        feature = "lang-go",
    ))]
    fn values(
        map: &std::collections::BTreeMap<String, codemmet_core::context::Collection>,
        name: &str,
    ) -> Vec<String> {
        map.get(name)
            .unwrap_or_else(|| panic!("no collection {name}; have {:?}", map.keys()))
            .items
            .iter()
            .map(|i| i.value.clone())
            .collect()
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn rust_enum_variants_extracted() {
        let map = cols(
            "rust",
            "enum Msg { Quit, Move { x: i32 }, Write(String) }\nfn f() { §\n}\n",
        );
        // The value is the match-ready pattern: a unit variant is bare, a struct
        // variant gets `{ .. }`, and a tuple variant gets `(..)`, so a generated
        // `match:Msg>arm*` compiles without naming each payload field. The bare
        // name stays reachable as the `name` member.
        assert_eq!(values(&map, "Msg"), ["Quit", "Move { .. }", "Write(..)"]);
        let msg = &map["Msg"];
        assert_eq!(msg.items[1].members["name"], "Move");
        assert_eq!(msg.items[2].members["name"], "Write");
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn rust_struct_fields_carry_types() {
        let map = cols(
            "rust",
            "struct Point { pub x: i32, y: f64, pub(crate) z: u8 }\nfn f() { §\n}\n",
        );
        assert_eq!(values(&map, "Point"), ["x", "y", "z"]);
        let point = &map["Point"];
        assert_eq!(point.items[0].members["type"], "i32");
        assert_eq!(point.items[1].members["type"], "f64");
        // Visibility is display-ready (trailing space); a private field
        // carries no `vis` at all.
        assert_eq!(point.items[0].members["vis"], "pub ");
        assert!(!point.items[1].members.contains_key("vis"));
        assert_eq!(point.items[2].members["vis"], "pub(crate) ");
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn rust_list_binding_in_scope() {
        let map = cols("rust", "fn f() {\n    let items = [1, 2, 3];\n    §\n}\n");
        assert_eq!(values(&map, "items"), ["1", "2", "3"]);
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn rust_vec_macro_and_scoping() {
        // `vec![…]` elements are read; a binding *below* the cursor is not.
        let map = cols(
            "rust",
            "fn f() {\n    let a = vec![x, y];\n    §\n    let b = [9];\n}\n",
        );
        assert_eq!(values(&map, "a"), ["x", "y"]);
        assert!(
            !map.contains_key("b"),
            "binding below cursor is out of scope"
        );
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_enums_unions_and_interfaces() {
        let map = cols(
            "typescript",
            "enum Color { Red, Green = 2 }\ntype Dir = \"n\" | \"s\";\ninterface Point { x: number; y?: string; }\n§\n",
        );
        assert_eq!(values(&map, "Color"), ["Red", "Green"]);
        // An assigned enum value is carried as the `value` member.
        assert!(!map["Color"].items[0].members.contains_key("value"));
        assert_eq!(map["Color"].items[1].members["value"], "2");
        assert_eq!(values(&map, "Dir"), ["\"n\"", "\"s\""]);
        assert_eq!(values(&map, "Point"), ["x", "y"]);
        assert_eq!(map["Point"].items[0].members["type"], "number");
        // `?` optionality is display-ready: the marker itself, or absent.
        assert!(!map["Point"].items[0].members.contains_key("optional"));
        assert_eq!(map["Point"].items[1].members["optional"], "?");
        assert_eq!(map["Point"].items[1].members["type"], "string");
    }

    #[cfg(feature = "lang-c")]
    #[test]
    fn c_enum_and_struct() {
        let map = cols(
            "c",
            "enum Color { RED, GREEN = 2, BLUE };\nstruct Point { int x; double y; };\nvoid f() { §; }\n",
        );
        assert_eq!(values(&map, "Color"), ["RED", "GREEN", "BLUE"]);
        // An enumerator's assigned value is carried as the `value` member.
        assert!(!map["Color"].items[0].members.contains_key("value"));
        assert_eq!(map["Color"].items[1].members["value"], "2");
        assert_eq!(values(&map, "Point"), ["x", "y"]);
        assert_eq!(map["Point"].items[0].members["type"], "int");
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn go_struct_fields_and_list() {
        let map = cols(
            "go",
            "type Point struct {\n\tX int\n\tY float64\n}\nfunc f() {\n\titems := []int{1, 2, 3}\n\t§\n}\n",
        );
        assert_eq!(values(&map, "Point"), ["X", "Y"]);
        assert_eq!(map["Point"].items[1].members["type"], "float64");
        assert_eq!(values(&map, "items"), ["1", "2", "3"]);
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn go_iota_const_group_reads_as_an_enum() {
        // An iota const group is Go's enum: keyed by the first spec's declared
        // type, items are every spec's names. A non-iota const group and an
        // untyped iota group contribute nothing.
        let map = cols(
            "go",
            "type Color int\n\nconst (\n\tRed Color = iota\n\tGreen\n\tBlue\n)\n\nconst (\n\tA = 1\n\tB = 2\n)\n\nfunc f() {\n\t§\n}\n",
        );
        assert_eq!(values(&map, "Color"), ["Red", "Green", "Blue"]);
        assert!(!map.contains_key("A"), "non-iota consts are not a group");
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn python_enum_dataclass_and_list() {
        let map = cols(
            "python",
            "class Color(Enum):\n    RED = 1\n    GREEN = 2\n\n@dataclass\nclass Point:\n    x: int\n    y: float = 0.5\n\nitems = [1, 2, 3]\n§\n",
        );
        assert_eq!(values(&map, "Color"), ["RED", "GREEN"]);
        // An enum member's assigned value is the `value` member.
        assert_eq!(map["Color"].items[0].members["value"], "1");
        assert_eq!(values(&map, "Point"), ["x", "y"]);
        assert_eq!(map["Point"].items[0].members["type"], "int");
        // An annotated field's initializer is the `default` member.
        assert!(!map["Point"].items[0].members.contains_key("default"));
        assert_eq!(map["Point"].items[1].members["default"], "0.5");
        assert_eq!(values(&map, "items"), ["1", "2", "3"]);
    }

    #[cfg(feature = "lang-javascript")]
    #[test]
    fn javascript_list_binding() {
        let map = cols("javascript", "const items = [1, 2, 3];\n§\n");
        assert_eq!(values(&map, "items"), ["1", "2", "3"]);
    }

    // ---- Fill-match-arms code action (codegen) -------------------------

    /// Fill targets at the `§` cursor.
    #[cfg(feature = "lang-rust")]
    fn fills(language: &str, fixture: &str) -> Vec<FillTarget> {
        let (doc, offset) = doc_at(language, fixture);
        TreeSitterProvider::new().fill_targets(&doc, offset)
    }

    /// Apply a fill target to the fixture text (marker stripped).
    #[cfg(feature = "lang-rust")]
    fn applied(fixture: &str, target: &FillTarget) -> String {
        let mut text = fixture.replacen('§', "", 1);
        text.replace_range(target.range.clone(), &target.new_text);
        text
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn fill_empty_match_from_enum() {
        let fixture = "enum Msg { Quit, Move, Write }\nfn f(m: Msg) {\n    match m {§}\n}\n";
        let targets = fills("rust", fixture);
        assert_eq!(targets.len(), 1, "one enum in scope");
        assert_eq!(targets[0].title, "Fill match arms from Msg");
        assert_eq!(
            applied(fixture, &targets[0]),
            "enum Msg { Quit, Move, Write }\nfn f(m: Msg) {\n    match m {\n        Quit => todo!(),\n        Move => todo!(),\n        Write => todo!(),\n    }\n}\n"
        );
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn fill_match_binds_payload_variants() {
        let fixture = "enum Msg { Quit, Write(String), Move { x: i32 } }\nfn f(m: Msg) {\n    match m {§}\n}\n";
        let targets = fills("rust", fixture);
        assert!(
            targets[0].new_text.contains("Write(..) => todo!(),")
                && targets[0].new_text.contains("Move { .. } => todo!(),"),
            "payload variants bind wildcards: {:?}",
            targets[0].new_text
        );
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn fill_match_skips_covered_variants_and_appends() {
        // One arm already present: only the missing variants are offered, and
        // they append after the existing arm rather than replacing the body.
        let fixture = "enum Msg { Quit, Move, Write }\nfn f(m: Msg) {\n    match m {\n        Quit => todo!(),§\n    }\n}\n";
        let targets = fills("rust", fixture);
        assert_eq!(targets.len(), 1);
        assert_eq!(
            targets[0].new_text,
            "\n        Move => todo!(),\n        Write => todo!(),"
        );
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn no_fill_outside_a_match() {
        let targets = fills("rust", "enum Msg { Quit }\nfn f() {\n    §\n}\n");
        assert!(targets.is_empty());
    }
}
