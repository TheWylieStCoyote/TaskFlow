//! Import-statement scanning and insertion points for `requires`.
//!
//! Per language this answers two questions the LSP layer needs to insert a
//! missing import (see `IMPORT_REQUIRES_PLAN.md`):
//!
//! * which import statements already exist (their text, for the conservative
//!   dedup match in `codemmet_core::lang::imports`), and
//! * the byte offset (a line start) where a new import line belongs — after
//!   the last existing import, or at the language's zero-import anchor
//!   (Rust: after inner attributes/`//!` docs; Go: after the `package`
//!   clause; Python: after the module docstring; C/C++: after `#pragma once`
//!   or an `#ifndef` guard's `#define`).
//!
//! Only *top-level* statements count (plus a leading C include guard's
//! children): a `use` inside a `mod`, or a Python import inside a function,
//! neither satisfies a requirement nor anchors insertion. JavaScript /
//! TypeScript are deferred (no import formatter yet), so they scan to
//! `None` like unsupported languages.

use codemmet_core::context::ImportScan;
use tree_sitter::Node;

/// Scan `root` for `language`'s imports, or `None` when the language has no
/// import support (JS/TS today).
pub fn scan(language: &str, root: Node<'_>, text: &str) -> Option<ImportScan> {
    match language {
        "rust" => Some(scan_statements(
            root,
            text,
            &["use_declaration"],
            rust_fallback,
        )),
        "python" => Some(scan_statements(
            root,
            text,
            &[
                "import_statement",
                "import_from_statement",
                "future_import_statement",
            ],
            python_fallback,
        )),
        "go" => Some(scan_go(root, text)),
        "c" | "cpp" => Some(scan_c(root, text)),
        _ => None,
    }
}

/// The named top-level children of `node`.
fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// Source text a node spans.
fn node_text<'a>(node: Node<'_>, text: &'a str) -> &'a str {
    &text[node.byte_range()]
}

/// The first line start at or after `offset`: `offset` itself when it
/// already sits at one (C preprocessor nodes include their trailing
/// newline), else just past the next newline, else end-of-file.
fn line_start_after(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    if offset == 0 || text.as_bytes()[offset - 1] == b'\n' {
        return offset;
    }
    text[offset..]
        .find('\n')
        .map_or(text.len(), |i| offset + i + 1)
}

/// The start of the line containing `offset`.
fn line_start_of(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())]
        .rfind('\n')
        .map_or(0, |i| i + 1)
}

/// Generic scan: top-level statements of the given kinds; insertion after
/// the last one, or at `fallback`'s zero-import anchor.
fn scan_statements(
    root: Node<'_>,
    text: &str,
    kinds: &[&str],
    fallback: fn(&[Node<'_>], &str) -> usize,
) -> ImportScan {
    let top = named_children(root);
    let imports: Vec<Node<'_>> = top
        .iter()
        .copied()
        .filter(|n| kinds.contains(&n.kind()))
        .collect();
    let insertion_offset = imports.last().map_or_else(
        || fallback(&top, text),
        |last| line_start_after(text, last.end_byte()),
    );
    ImportScan {
        statements: imports
            .iter()
            .map(|n| node_text(*n, text).to_owned())
            .collect(),
        insertion_offset,
        grouped_insertion_offset: None,
        leading_blank: false,
    }
}

/// Rust zero-import anchor: after leading inner attributes (`#![…]`) and
/// inner doc comments (`//!`, `/*! … */`), else the top of the file.
///
/// Outer doc comments (`///`) and plain `//` comments stop the skip — they
/// attach to the item below them, so the import goes above.
fn rust_fallback(top: &[Node<'_>], text: &str) -> usize {
    let mut offset = 0;
    for node in top {
        let skip = match node.kind() {
            "inner_attribute_item" => true,
            "line_comment" => node_text(*node, text).starts_with("//!"),
            "block_comment" => node_text(*node, text).starts_with("/*!"),
            _ => false,
        };
        if !skip {
            break;
        }
        offset = line_start_after(text, node.end_byte());
    }
    offset
}

/// Python zero-import anchor: after leading comments (shebang, coding
/// cookie — both must stay on their original lines) and the module
/// docstring, else the top of the file.
fn python_fallback(top: &[Node<'_>], text: &str) -> usize {
    let mut offset = 0;
    for node in top {
        let is_docstring = node.kind() == "expression_statement"
            && node
                .named_child(0)
                .is_some_and(|child| child.kind() == "string");
        if node.kind() != "comment" && !is_docstring {
            break;
        }
        offset = line_start_after(text, node.end_byte());
        if is_docstring {
            // Only the first statement can be the module docstring.
            break;
        }
    }
    offset
}

/// Go: standalone and grouped `import` declarations; a new import prefers
/// joining the last grouped block. Zero imports insert after the `package`
/// clause with a separating blank line.
fn scan_go(root: Node<'_>, text: &str) -> ImportScan {
    let top = named_children(root);
    let decls: Vec<Node<'_>> = top
        .iter()
        .copied()
        .filter(|n| n.kind() == "import_declaration")
        .collect();
    let (insertion_offset, leading_blank) = decls.last().map_or_else(
        || {
            let after_package = top
                .iter()
                .find(|n| n.kind() == "package_clause")
                .map_or(0, |pkg| line_start_after(text, pkg.end_byte()));
            (after_package, after_package > 0)
        },
        |last| (line_start_after(text, last.end_byte()), false),
    );
    // The last grouped block whose `)` sits on its own line: a new import
    // can slot in as a line right before it. (A single-line `import ( … )`
    // offers no line to join; fall back to a standalone declaration.)
    let grouped_insertion_offset = decls.iter().rev().find_map(|decl| {
        let list = named_children(*decl)
            .into_iter()
            .find(|n| n.kind() == "import_spec_list")?;
        let mut cursor = list.walk();
        let close = list.children(&mut cursor).find(|c| c.kind() == ")")?;
        let close_line = line_start_of(text, close.start_byte());
        (close_line > line_start_of(text, list.start_byte())).then_some(close_line)
    });
    ImportScan {
        statements: decls
            .iter()
            .map(|n| node_text(*n, text).to_owned())
            .collect(),
        insertion_offset,
        grouped_insertion_offset,
        leading_blank,
    }
}

/// C/C++: `#include` directives at the top level, descending into a leading
/// include guard (`#ifndef`/`#define`) whose children are the effective top
/// level. Zero includes insert after `#pragma once` / the guard's
/// `#define`, else at the top of the file.
fn scan_c(root: Node<'_>, text: &str) -> ImportScan {
    let top = named_children(root);
    let first_code = top.iter().copied().find(|n| n.kind() != "comment");
    let (nodes, fallback) = match first_code {
        Some(node) if node.kind() == "preproc_ifdef" => {
            // A leading `#ifndef GUARD` wraps the whole file: scan inside it,
            // anchoring zero-include insertion after its `#define` line.
            let inner = named_children(node);
            let anchor = inner
                .iter()
                .find(|n| n.kind() == "preproc_def")
                .map_or_else(|| node.start_byte(), Node::end_byte);
            (inner, line_start_after(text, anchor))
        }
        Some(node)
            if node.kind() == "preproc_call" && node_text(node, text).starts_with("#pragma") =>
        {
            (top.clone(), line_start_after(text, node.end_byte()))
        }
        _ => (top.clone(), 0),
    };
    let includes: Vec<Node<'_>> = nodes
        .iter()
        .copied()
        .filter(|n| n.kind() == "preproc_include")
        .collect();
    let insertion_offset = includes
        .last()
        .map_or(fallback, |last| line_start_after(text, last.end_byte()));
    ImportScan {
        statements: includes
            .iter()
            .map(|n| node_text(*n, text).to_owned())
            .collect(),
        insertion_offset,
        grouped_insertion_offset: None,
        leading_blank: false,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "lang-rust",
        feature = "lang-python",
        feature = "lang-c",
        feature = "lang-cpp",
        feature = "lang-go",
        feature = "lang-typescript",
    ))]
    use codemmet_core::context::{ContextProvider as _, DocSnapshot, ImportScan};

    /// Scan a fixture through the public provider seam.
    #[cfg(any(
        feature = "lang-rust",
        feature = "lang-python",
        feature = "lang-c",
        feature = "lang-cpp",
        feature = "lang-go",
        feature = "lang-typescript",
    ))]
    fn scan_doc(language: &str, text: &str) -> Option<ImportScan> {
        let doc = DocSnapshot {
            language: language.into(),
            text: text.into(),
        };
        crate::TreeSitterProvider::new().import_scan(&doc)
    }

    /// Byte offset where `needle` starts (fixtures place it at a line start).
    #[cfg(any(
        feature = "lang-rust",
        feature = "lang-python",
        feature = "lang-c",
        feature = "lang-cpp",
        feature = "lang-go",
    ))]
    fn at(text: &str, needle: &str) -> usize {
        text.find(needle).expect("fixture contains needle")
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn rust_inserts_after_last_top_level_use() {
        let text = "use std::fs;\nuse std::io::Read;\n\nfn main() {}\n";
        let scan = scan_doc("rust", text).expect("rust scans");
        assert_eq!(scan.statements, ["use std::fs;", "use std::io::Read;"]);
        assert_eq!(scan.insertion_offset, at(text, "\nfn main"));
        assert!(scan.grouped_insertion_offset.is_none());
        assert!(!scan.leading_blank);
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn rust_nested_use_neither_counts_nor_anchors() {
        let text = "fn f() {}\nmod m {\n    use std::fs;\n}\n";
        let scan = scan_doc("rust", text).expect("rust scans");
        assert!(scan.statements.is_empty());
        assert_eq!(scan.insertion_offset, 0);
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn rust_zero_imports_skips_inner_attrs_and_docs() {
        let text = "//! Module docs.\n#![allow(dead_code)]\n\nfn main() {}\n";
        let scan = scan_doc("rust", text).expect("rust scans");
        assert!(scan.statements.is_empty());
        // After the skipped prelude: the blank line before `fn main`.
        assert_eq!(scan.insertion_offset, at(text, "\nfn main"));
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn rust_outer_doc_comment_stops_the_skip() {
        // `///` attaches to the item below: the import goes above it.
        let text = "/// Docs for main.\nfn main() {}\n";
        let scan = scan_doc("rust", text).expect("rust scans");
        assert_eq!(scan.insertion_offset, 0);
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn python_inserts_after_last_import_including_future() {
        let text = "from __future__ import annotations\nimport os\n\ndef f():\n    pass\n";
        let scan = scan_doc("python", text).expect("python scans");
        assert_eq!(
            scan.statements,
            ["from __future__ import annotations", "import os"]
        );
        assert_eq!(scan.insertion_offset, at(text, "\ndef f"));
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn python_zero_imports_skips_shebang_and_docstring() {
        let text = "#!/usr/bin/env python\n\"\"\"Module docstring.\"\"\"\n\ndef f():\n    pass\n";
        let scan = scan_doc("python", text).expect("python scans");
        assert!(scan.statements.is_empty());
        assert_eq!(scan.insertion_offset, at(text, "\ndef f"));
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn python_plain_code_inserts_at_top() {
        let scan = scan_doc("python", "x = 1\n").expect("python scans");
        assert_eq!(scan.insertion_offset, 0);
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn go_grouped_block_offers_a_grouped_insertion_point() {
        let text = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {}\n";
        let scan = scan_doc("go", text).expect("go scans");
        assert_eq!(scan.statements.len(), 1);
        assert!(scan.statements[0].contains("\"fmt\""));
        // The grouped point is the line holding `)`.
        assert_eq!(scan.grouped_insertion_offset, Some(at(text, ")\n")));
        assert_eq!(scan.insertion_offset, at(text, "\nfunc main"));
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn go_zero_imports_anchor_after_package_with_blank() {
        let text = "package main\n\nfunc main() {}\n";
        let scan = scan_doc("go", text).expect("go scans");
        assert!(scan.statements.is_empty());
        assert_eq!(scan.insertion_offset, at(text, "\nfunc main"));
        assert!(scan.leading_blank);
        assert!(scan.grouped_insertion_offset.is_none());
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn go_single_line_group_offers_no_grouped_point() {
        let text = "package main\n\nimport (\"fmt\")\n\nfunc main() {}\n";
        let scan = scan_doc("go", text).expect("go scans");
        assert!(scan.grouped_insertion_offset.is_none());
        assert_eq!(scan.insertion_offset, at(text, "\nfunc main"));
    }

    #[cfg(feature = "lang-c")]
    #[test]
    fn c_inserts_after_last_include() {
        let text = "#include <stdio.h>\n#include <stdlib.h>\n\nint main(void) {}\n";
        let scan = scan_doc("c", text).expect("c scans");
        assert_eq!(scan.statements.len(), 2);
        assert!(scan.statements[1].contains("<stdlib.h>"));
        assert_eq!(scan.insertion_offset, at(text, "\nint main"));
    }

    #[cfg(feature = "lang-c")]
    #[test]
    fn c_pragma_once_anchors_zero_include_insertion() {
        let text = "#pragma once\n\nvoid f(void);\n";
        let scan = scan_doc("c", text).expect("c scans");
        assert!(scan.statements.is_empty());
        assert_eq!(scan.insertion_offset, at(text, "\nvoid f"));
    }

    #[cfg(feature = "lang-c")]
    #[test]
    fn c_include_guard_scans_and_anchors_inside() {
        let text =
            "#ifndef FOO_H\n#define FOO_H\n\n#include <stddef.h>\n\nvoid f(void);\n\n#endif\n";
        let scan = scan_doc("c", text).expect("c scans");
        assert_eq!(scan.statements.len(), 1);
        assert!(scan.statements[0].contains("<stddef.h>"));
        assert_eq!(scan.insertion_offset, at(text, "\nvoid f"));

        // Without includes, insertion follows the guard's `#define`.
        let empty = "#ifndef FOO_H\n#define FOO_H\n\nvoid f(void);\n\n#endif\n";
        let scan = scan_doc("c", empty).expect("c scans");
        assert_eq!(scan.insertion_offset, at(empty, "\nvoid f"));
    }

    #[cfg(feature = "lang-cpp")]
    #[test]
    fn cpp_shares_the_c_scan() {
        let text = "#include <vector>\n\nint main() {}\n";
        let scan = scan_doc("cpp", text).expect("cpp scans");
        assert_eq!(scan.statements.len(), 1);
        assert_eq!(scan.insertion_offset, at(text, "\nint main"));
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_is_deferred_and_scans_to_none() {
        assert!(scan_doc("typescript", "import { x } from \"y\";\n").is_none());
    }
}
