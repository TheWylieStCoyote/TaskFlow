//! Buffer-sourced [`Collection`]s for data-driven repeats.
//!
//! The expansion engine resolves a data-driven repeat (`arm*@Msg`, or a bare
//! `arm*` under `match:Msg`) purely by *name*: it looks the name up in
//! [`ExpansionContext::collections`](codemmet_core::context::ExpansionContext).
//! So this module's whole job is to gather every named collection it can see in
//! the buffer and key it by that name:
//!
//! * **enum variants** (CX2) — keyed by the enum/type name (`Msg` → its
//!   variants). Visible file-wide, since a definition can sit below its use.
//! * **struct/record fields** (CX3) — keyed by the type name (`Point` → its
//!   fields), each field carrying a `type` member for `${value.type}`. Also
//!   file-wide.
//! * **in-scope list bindings** (CX1) — keyed by the variable name
//!   (`items` → its elements). Only bindings lexically in scope at the cursor
//!   and declared textually before it are offered; on a name clash the nearest
//!   enclosing binding wins.
//!
//! Extraction is deliberately shape-directed rather than type-aware: it reads
//! what the syntax tree literally says (an enum's written variants, a list
//! literal's written elements). Inferring a variable's *type* to find its
//! definition is out of scope — the user names the source instead
//! (`arm*@Msg`). See `CONTEXT_CODEGEN_PLAN.md`.

use std::collections::BTreeMap;

use codemmet_core::context::{Collection, Item};
use tree_sitter::Node;

/// Gather every named collection visible at `offset` in `root`.
pub fn collections(
    language: &str,
    root: Node<'_>,
    text: &str,
    offset: usize,
) -> BTreeMap<String, Collection> {
    let mut map = BTreeMap::new();
    match language {
        "rust" => rust_defs(root, text, &mut map),
        "c" | "cpp" => c_defs(root, text, &mut map),
        "typescript" => ts_defs(root, text, &mut map),
        "go" => go_defs(root, text, &mut map),
        "python" => python_defs(root, text, &mut map),
        // JavaScript has no enum/struct definitions; only list bindings apply.
        _ => {}
    }
    add_list_bindings(language, root, text, offset, &mut map);
    map
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Source text a node spans.
fn node_text<'a>(node: Node<'_>, text: &'a str) -> &'a str {
    &text[node.byte_range()]
}

/// Text of a node's named field, if present.
fn field_text<'a>(node: Node<'_>, field: &str, text: &'a str) -> Option<&'a str> {
    node.child_by_field_name(field).map(|c| node_text(c, text))
}

/// Every named descendant of `root` (inclusive) whose kind is in `kinds`, in
/// document (pre-)order.
fn descendants_of<'t>(root: Node<'t>, kinds: &[&str]) -> Vec<Node<'t>> {
    let mut out = Vec::new();
    collect_descendants(root, kinds, &mut out);
    out
}

fn collect_descendants<'t>(node: Node<'t>, kinds: &[&str], out: &mut Vec<Node<'t>>) {
    if node.is_named() && kinds.contains(&node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_descendants(child, kinds, out);
    }
}

/// Named children of a node's field (empty when the field is absent).
fn field_children<'t>(node: Node<'t>, field: &str) -> Vec<Node<'t>> {
    let Some(container) = node.child_by_field_name(field) else {
        return Vec::new();
    };
    let mut cursor = container.walk();
    container.named_children(&mut cursor).collect()
}

/// A field's declared type, cleaned for display: a leading `:` (TypeScript's
/// `type_annotation` includes the colon) and surrounding whitespace stripped.
fn clean_type(raw: &str) -> String {
    raw.trim().trim_start_matches(':').trim().to_owned()
}

/// Insert a collection under `name` unless a definition already claimed it or
/// the item list is empty. File-wide definitions overwrite; scoped list
/// bindings use [`BTreeMap::entry`] so the nearest one wins.
fn insert_def(map: &mut BTreeMap<String, Collection>, name: &str, items: Vec<Item>) {
    if !items.is_empty() {
        map.insert(name.to_owned(), Collection { items });
    }
}

// ---------------------------------------------------------------------------
// Enum variants (CX2)
// ---------------------------------------------------------------------------

/// Enum-like definitions whose members are the direct named children of a
/// container field. A member's value is its `name` field when it has one
/// (Rust `enum_variant`, C `enumerator`, TS `enum_assignment`), else the
/// member's own text (a bare TS enum member, a union `literal_type`).
fn extract_enum(
    root: Node<'_>,
    text: &str,
    def_kind: &str,
    container_field: &str,
    member_kinds: &[&str],
    map: &mut BTreeMap<String, Collection>,
) {
    for def in descendants_of(root, &[def_kind]) {
        let Some(name) = field_text(def, "name", text) else {
            continue;
        };
        let items = field_children(def, container_field)
            .into_iter()
            .filter(|m| member_kinds.contains(&m.kind()))
            .map(|m| Item::new(field_text(m, "name", text).unwrap_or_else(|| node_text(m, text))))
            .collect();
        insert_def(map, name, items);
    }
}

// ---------------------------------------------------------------------------
// Per-language definition passes (enums + structs)
// ---------------------------------------------------------------------------

fn rust_defs(root: Node<'_>, text: &str, map: &mut BTreeMap<String, Collection>) {
    extract_enum(root, text, "enum_item", "body", &["enum_variant"], map);
    for def in descendants_of(root, &["struct_item"]) {
        let Some(name) = field_text(def, "name", text) else {
            continue;
        };
        let items = field_children(def, "body")
            .into_iter()
            .filter(|f| f.kind() == "field_declaration")
            .filter_map(|f| named_field_item(f, text))
            .collect();
        insert_def(map, name, items);
    }
}

fn ts_defs(root: Node<'_>, text: &str, map: &mut BTreeMap<String, Collection>) {
    extract_enum(
        root,
        text,
        "enum_declaration",
        "body",
        &["property_identifier", "enum_assignment"],
        map,
    );
    // `type Dir = "n" | "s"` — a union of literals reads as an enum.
    extract_enum(
        root,
        text,
        "type_alias_declaration",
        "value",
        &["literal_type"],
        map,
    );
    for def in descendants_of(root, &["interface_declaration"]) {
        let Some(name) = field_text(def, "name", text) else {
            continue;
        };
        let items = field_children(def, "body")
            .into_iter()
            .filter(|f| f.kind() == "property_signature")
            .filter_map(|f| named_field_item(f, text))
            .collect();
        insert_def(map, name, items);
    }
}

fn c_defs(root: Node<'_>, text: &str, map: &mut BTreeMap<String, Collection>) {
    extract_enum(root, text, "enum_specifier", "body", &["enumerator"], map);
    for def in descendants_of(root, &["struct_specifier", "union_specifier"]) {
        let Some(name) = field_text(def, "name", text) else {
            continue;
        };
        let items = field_children(def, "body")
            .into_iter()
            .filter(|f| f.kind() == "field_declaration")
            .filter_map(|f| c_field_item(f, text))
            .collect();
        insert_def(map, name, items);
    }
}

fn go_defs(root: Node<'_>, text: &str, map: &mut BTreeMap<String, Collection>) {
    // Go structs live under `type_spec name: … type: (struct_type …)`.
    for spec in descendants_of(root, &["type_spec"]) {
        let Some(name) = field_text(spec, "name", text) else {
            continue;
        };
        let Some(ty) = spec.child_by_field_name("type") else {
            continue;
        };
        if ty.kind() != "struct_type" {
            continue;
        }
        let mut items = Vec::new();
        for field in descendants_of(ty, &["field_declaration"]) {
            let type_text = field_text(field, "type", text).map(clean_type);
            // A Go field declaration can name several fields at once
            // (`X, Y int`); each `name` child becomes its own item.
            let mut cursor = field.walk();
            for child in field.children_by_field_name("name", &mut cursor) {
                let mut item = Item::new(node_text(child, text));
                if let Some(t) = &type_text {
                    item.members.insert("type".to_owned(), t.clone());
                }
                items.push(item);
            }
        }
        insert_def(map, name, items);
    }
}

fn python_defs(root: Node<'_>, text: &str, map: &mut BTreeMap<String, Collection>) {
    for class in descendants_of(root, &["class_definition"]) {
        let Some(name) = field_text(class, "name", text) else {
            continue;
        };
        let is_enum = field_children(class, "superclasses")
            .iter()
            .any(|s| node_text(*s, text).ends_with("Enum"));
        let mut items = Vec::new();
        for stmt in field_children(class, "body") {
            if stmt.kind() != "expression_statement" {
                continue;
            }
            let Some(assign) = stmt.named_child(0).filter(|a| a.kind() == "assignment") else {
                continue;
            };
            let Some(target) = assign.child_by_field_name("left") else {
                continue;
            };
            if target.kind() != "identifier" {
                continue;
            }
            // An enum's members are its assigned names; a dataclass's fields
            // are its *annotated* attributes (`x: int`). A plain assignment in
            // a dataclass (a default without annotation) is skipped.
            let annotated = assign.child_by_field_name("type");
            if !is_enum && annotated.is_none() {
                continue;
            }
            let mut item = Item::new(node_text(target, text));
            if let Some(ty) = annotated {
                item.members
                    .insert("type".to_owned(), clean_type(node_text(ty, text)));
            }
            items.push(item);
        }
        insert_def(map, name, items);
    }
}

/// A struct field whose name is in a `name` field and type in a `type` field
/// (Rust, Go, TypeScript). The `type` member is omitted when absent.
fn named_field_item(field: Node<'_>, text: &str) -> Option<Item> {
    let name = field_text(field, "name", text)?;
    let mut item = Item::new(name);
    if let Some(ty) = field_text(field, "type", text) {
        item.members.insert("type".to_owned(), clean_type(ty));
    }
    Some(item)
}

/// A C/C++ struct field: the name lives in a `declarator` (a bare
/// `field_identifier`, or nested inside a pointer/array declarator), the type
/// in a `type` field.
fn c_field_item(field: Node<'_>, text: &str) -> Option<Item> {
    let declarator = field.child_by_field_name("declarator")?;
    let ident = if declarator.kind() == "field_identifier" {
        declarator
    } else {
        descendants_of(declarator, &["field_identifier"])
            .into_iter()
            .next()?
    };
    let mut item = Item::new(node_text(ident, text));
    if let Some(ty) = field_text(field, "type", text) {
        item.members.insert("type".to_owned(), clean_type(ty));
    }
    Some(item)
}

// ---------------------------------------------------------------------------
// In-scope list bindings (CX1)
// ---------------------------------------------------------------------------

/// Add list/array bindings that are lexically in scope at `offset`.
///
/// Walks enclosing scopes from the cursor outward; within each, any statement
/// starting before the cursor that binds a list literal contributes a
/// collection. The nearest binding wins a name clash (scopes are visited
/// innermost first and [`BTreeMap::entry`] keeps the first insert).
fn add_list_bindings(
    language: &str,
    root: Node<'_>,
    text: &str,
    offset: usize,
    map: &mut BTreeMap<String, Collection>,
) {
    let mut scope = root.named_descendant_for_byte_range(offset, offset);
    while let Some(node) = scope {
        for stmt in scope_statements(node) {
            if stmt.start_byte() >= offset {
                continue;
            }
            for (name, items) in list_bindings(language, stmt, text) {
                if !items.is_empty() && !map.contains_key(&name) {
                    map.insert(name, Collection { items });
                }
            }
        }
        scope = node.parent();
    }
}

/// The statement nodes directly inside a scope, flattening grouping wrappers
/// that are not themselves scopes (Go nests statements under an extra
/// `statement_list` inside each `block`).
fn scope_statements(node: Node<'_>) -> Vec<Node<'_>> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "statement_list" {
            let mut inner = child.walk();
            out.extend(child.named_children(&mut inner));
        } else {
            out.push(child);
        }
    }
    out
}

/// The list bindings a single statement introduces, as `(name, elements)`.
fn list_bindings(language: &str, stmt: Node<'_>, text: &str) -> Vec<(String, Vec<Item>)> {
    match language {
        "rust" => rust_list(stmt, text),
        "python" => python_list(stmt, text),
        "javascript" | "typescript" => js_list(stmt, text),
        "go" => go_list(stmt, text),
        "c" | "cpp" => c_list(stmt, text),
        _ => Vec::new(),
    }
}

/// Elements of a literal-container node, each as an [`Item`].
fn elements(container: Node<'_>, text: &str) -> Vec<Item> {
    let mut cursor = container.walk();
    container
        .named_children(&mut cursor)
        .map(|e| Item::new(node_text(e, text)))
        .collect()
}

fn rust_list(stmt: Node<'_>, text: &str) -> Vec<(String, Vec<Item>)> {
    if stmt.kind() != "let_declaration" {
        return Vec::new();
    }
    let Some(pattern) = stmt.child_by_field_name("pattern") else {
        return Vec::new();
    };
    if pattern.kind() != "identifier" {
        return Vec::new();
    }
    let name = node_text(pattern, text).to_owned();
    let Some(value) = stmt.child_by_field_name("value") else {
        return Vec::new();
    };
    match value.kind() {
        "array_expression" => vec![(name, elements(value, text))],
        // `vec![a, b, c]` — the elements are the named children of the macro's
        // token tree (commas are anonymous tokens).
        "macro_invocation" if field_text(value, "macro", text) == Some("vec") => value
            .named_children(&mut value.walk())
            .find(|c| c.kind() == "token_tree")
            .map_or_else(Vec::new, |tt| vec![(name.clone(), elements(tt, text))]),
        _ => Vec::new(),
    }
}

fn python_list(stmt: Node<'_>, text: &str) -> Vec<(String, Vec<Item>)> {
    if stmt.kind() != "expression_statement" {
        return Vec::new();
    }
    let Some(assign) = stmt.named_child(0).filter(|a| a.kind() == "assignment") else {
        return Vec::new();
    };
    let (Some(left), Some(right)) = (
        assign.child_by_field_name("left"),
        assign.child_by_field_name("right"),
    ) else {
        return Vec::new();
    };
    if left.kind() != "identifier" || right.kind() != "list" {
        return Vec::new();
    }
    vec![(node_text(left, text).to_owned(), elements(right, text))]
}

fn js_list(stmt: Node<'_>, text: &str) -> Vec<(String, Vec<Item>)> {
    if !matches!(stmt.kind(), "lexical_declaration" | "variable_declaration") {
        return Vec::new();
    }
    let mut out = Vec::new();
    for decl in descendants_of(stmt, &["variable_declarator"]) {
        let (Some(name), Some(value)) = (
            decl.child_by_field_name("name"),
            decl.child_by_field_name("value"),
        ) else {
            continue;
        };
        if name.kind() == "identifier" && value.kind() == "array" {
            out.push((node_text(name, text).to_owned(), elements(value, text)));
        }
    }
    out
}

fn go_list(stmt: Node<'_>, text: &str) -> Vec<(String, Vec<Item>)> {
    // Both `var items = []T{…}` and `items := []T{…}`.
    let specs = match stmt.kind() {
        "var_declaration" => descendants_of(stmt, &["var_spec"]),
        "short_var_declaration" => vec![stmt],
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for spec in specs {
        // `var_spec` names via a `name` field; `:=` names via a `left`
        // expression list of identifiers.
        let names: Vec<Node<'_>> = if spec.kind() == "short_var_declaration" {
            spec.child_by_field_name("left")
                .map(|l| l.named_children(&mut l.walk()).collect())
                .unwrap_or_default()
        } else {
            spec.children_by_field_name("name", &mut spec.walk())
                .collect()
        };
        let value_field = if spec.kind() == "short_var_declaration" {
            "right"
        } else {
            "value"
        };
        let Some(value) = spec.child_by_field_name(value_field) else {
            continue;
        };
        // The value is an expression list; take its first composite literal.
        let Some(composite) = descendants_of(value, &["composite_literal"])
            .into_iter()
            .next()
        else {
            continue;
        };
        let Some(body) = composite.child_by_field_name("body") else {
            continue;
        };
        if let Some(name) = names.first() {
            out.push((node_text(*name, text).to_owned(), elements(body, text)));
        }
    }
    out
}

fn c_list(stmt: Node<'_>, text: &str) -> Vec<(String, Vec<Item>)> {
    if stmt.kind() != "declaration" {
        return Vec::new();
    }
    let mut out = Vec::new();
    for init in descendants_of(stmt, &["init_declarator"]) {
        let (Some(declarator), Some(value)) = (
            init.child_by_field_name("declarator"),
            init.child_by_field_name("value"),
        ) else {
            continue;
        };
        if value.kind() != "initializer_list" {
            continue;
        }
        // The bound name is the identifier inside the (possibly array)
        // declarator.
        let ident = if declarator.kind() == "identifier" {
            Some(declarator)
        } else {
            descendants_of(declarator, &["identifier"])
                .into_iter()
                .next()
        };
        if let Some(ident) = ident {
            out.push((node_text(ident, text).to_owned(), elements(value, text)));
        }
    }
    out
}
