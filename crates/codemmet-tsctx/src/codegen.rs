//! Code-generation edits behind editor code actions.
//!
//! Where [`extract`](crate::extract) gathers collections for *typing* a
//! data-driven abbreviation, this module produces ready-to-apply edits for
//! *existing* code — the discoverable counterpart. Today it fills a Rust
//! `match` from an in-scope `enum`: put the cursor in `match msg { }` and the
//! editor offers "Fill match arms from `Msg`", inserting one arm per variant.
//!
//! The edits are plain text (no tabstops), so any LSP client applies them
//! without cooperating with the codemmet plugin. Rust-only for now; the
//! per-language shape lives here, mirroring [`extract`].

use std::fmt::Write as _;

use codemmet_core::context::FillTarget;
use tree_sitter::Node;

/// "Fill match arms" targets for the Rust `match` enclosing `offset`, one per
/// in-scope enum that still has unmatched variants. Empty when the cursor is
/// not inside a `match`, or every enum is already fully covered.
pub fn match_arm_fills(
    root: Node<'_>,
    text: &str,
    offset: usize,
    indent_unit: &str,
) -> Vec<FillTarget> {
    let Some(mtch) = enclosing_match(root, offset) else {
        return Vec::new();
    };
    let Some(block) = mtch.child_by_field_name("body") else {
        return Vec::new();
    };
    let arms = arm_nodes(block);
    let existing: Vec<String> = arms
        .iter()
        .filter_map(|a| a.child_by_field_name("pattern"))
        .map(|p| head_of(node_text(p, text)))
        .collect();

    let line_indent = line_indent(text, mtch.start_byte());
    let arm_indent = format!("{line_indent}{indent_unit}");

    let mut out = Vec::new();
    for (name, variants) in rust_enums(root, text) {
        let missing: Vec<String> = variants
            .into_iter()
            .filter(|v| !existing.contains(&head_of(v)))
            .collect();
        if missing.is_empty() {
            continue;
        }
        let mut arms_text = String::new();
        for v in &missing {
            let _ = write!(arms_text, "\n{arm_indent}{v} => todo!(),");
        }
        let (range, new_text) = if let Some(last) = arms.last() {
            // Append after the last existing arm; its trailing `\n}` stays.
            (last.end_byte()..last.end_byte(), arms_text)
        } else {
            // Empty body: replace the gap between the braces with clean arms
            // and a closing newline+dedent.
            let (Some(open), Some(close)) = (brace(block, "{"), brace(block, "}")) else {
                continue;
            };
            (
                open.end_byte()..close.start_byte(),
                format!("{arms_text}\n{line_indent}"),
            )
        };
        out.push(FillTarget {
            title: format!("Fill match arms from {name}"),
            range,
            new_text,
        });
    }
    out
}

/// The innermost `match_expression` covering `offset`, if any.
fn enclosing_match(root: Node<'_>, offset: usize) -> Option<Node<'_>> {
    let mut node = root.named_descendant_for_byte_range(offset, offset)?;
    loop {
        if node.kind() == "match_expression" {
            return Some(node);
        }
        node = node.parent()?;
    }
}

/// The `match_arm` children of a `match_block`, in order.
fn arm_nodes(block: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = block.walk();
    block
        .named_children(&mut cursor)
        .filter(|c| c.kind() == "match_arm")
        .collect()
}

/// The `{` or `}` token child of a `match_block`.
fn brace<'t>(block: Node<'t>, which: &str) -> Option<Node<'t>> {
    let mut cursor = block.walk();
    block.children(&mut cursor).find(|c| c.kind() == which)
}

/// Every `enum` in the file as `(name, variant match-patterns)`. A unit variant
/// is bare (`Quit`), a tuple variant binds a wildcard (`Write(..)`), a struct
/// variant likewise (`Move { .. }`), so generated arms compile.
fn rust_enums(root: Node<'_>, text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
        if node.kind() != "enum_item" {
            continue;
        }
        let Some(name) = field_text(node, "name", text) else {
            continue;
        };
        let Some(body) = node.child_by_field_name("body") else {
            continue;
        };
        let mut variants = Vec::new();
        let mut vc = body.walk();
        for v in body.named_children(&mut vc) {
            if v.kind() != "enum_variant" {
                continue;
            }
            if let Some(pattern) = variant_pattern(v, text) {
                variants.push(pattern);
            }
        }
        if !variants.is_empty() {
            out.push((name.to_owned(), variants));
        }
    }
    out
}

/// A variant's match pattern (see [`rust_enums`]).
fn variant_pattern(variant: Node<'_>, text: &str) -> Option<String> {
    let name = field_text(variant, "name", text)?;
    Some(
        match variant.child_by_field_name("body").map(|b| b.kind()) {
            Some("ordered_field_declaration_list") => format!("{name}(..)"),
            Some("field_declaration_list") => format!("{name} {{ .. }}"),
            _ => name.to_owned(),
        },
    )
}

/// The leading identifier of a pattern, for comparing coverage (`Write(..)` and
/// an existing `Write(s)` arm share the head `Write`; `_` heads as empty).
fn head_of(pattern: &str) -> String {
    pattern
        .trim_start()
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// The leading whitespace of the line containing `offset`.
fn line_indent(text: &str, offset: usize) -> String {
    let start = text[..offset.min(text.len())]
        .rfind('\n')
        .map_or(0, |i| i + 1);
    text[start..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

fn node_text<'a>(node: Node<'_>, text: &'a str) -> &'a str {
    &text[node.byte_range()]
}

fn field_text<'a>(node: Node<'_>, field: &str, text: &'a str) -> Option<&'a str> {
    node.child_by_field_name(field).map(|c| node_text(c, text))
}
