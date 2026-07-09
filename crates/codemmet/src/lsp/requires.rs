//! Building the plain-text edit that inserts an expansion's missing imports.
//!
//! Bridges `Expansion.requires` (raw specs collected by the renderer) and
//! the provider's [`ImportScan`]: format each spec for the language, drop
//! the ones an existing import statement already satisfies, and emit ONE
//! zero-width [`TextEdit`] at the scan's insertion point carrying every
//! missing line. Completion attaches it as `additionalTextEdits`; the
//! `codemmet.expand`/`codemmet.wrap` commands return it as `extraEdits`.

use codemmet_core::context::ImportScan;
use codemmet_core::lang::imports::{self, ImportSpec};
use lsp_types::{Position, Range, TextEdit};

use super::line_index::{LineIndex, PositionEncoding};

/// The import-insertion edit for `requires` against `scan`, or `None` when
/// nothing is missing.
///
/// At most one edit comes back: multiple zero-width edits at the same
/// position would be ambiguous to order, so every missing line joins a
/// single insertion. `None` when every requirement is satisfied, no spec
/// parses (unsupported language), or the insertion point lands on
/// `main_start.line` — LSP forbids additional edits overlapping the main
/// edit, and a zero-width insert at the main edit's own line is ambiguous
/// across clients, so the safe direction is to drop the import and let the
/// expansion stand alone.
pub fn import_edit(
    language: &str,
    requires: &[String],
    scan: &ImportScan,
    index: &LineIndex,
    text: &str,
    encoding: PositionEncoding,
    main_start: Position,
) -> Option<TextEdit> {
    let missing: Vec<ImportSpec> = requires
        .iter()
        .filter_map(|spec| imports::parse_spec(language, spec).ok())
        .filter(|spec| !scan.statements.iter().any(|s| spec.satisfied_by(s)))
        .collect();
    if missing.is_empty() {
        return None;
    }

    // Go: join an existing grouped `import ( … )` block when one is there.
    let (offset, lines): (usize, Vec<String>) = match scan.grouped_insertion_offset {
        Some(grouped) if missing.iter().all(|s| s.grouped_line().is_some()) => (
            grouped,
            missing
                .iter()
                .filter_map(ImportSpec::grouped_line)
                .collect(),
        ),
        _ => (
            scan.insertion_offset,
            missing.iter().map(ImportSpec::line).collect(),
        ),
    };

    let position = index.position(text, offset.min(text.len()), encoding);
    if position.line == main_start.line {
        return None;
    }

    let mut new_text = String::new();
    // Inserting at an end-of-file that lacks a trailing newline: open a
    // fresh line first so the import does not glue onto the last line.
    if offset >= text.len() && !text.is_empty() && !text.ends_with('\n') {
        new_text.push('\n');
    }
    if scan.leading_blank {
        new_text.push('\n');
    }
    for line in &lines {
        new_text.push_str(line);
        new_text.push('\n');
    }
    Some(TextEdit {
        range: Range {
            start: position,
            end: position,
        },
        new_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(
        language: &str,
        requires: &[&str],
        scan: &ImportScan,
        text: &str,
        main_line: u32,
    ) -> Option<TextEdit> {
        let requires: Vec<String> = requires.iter().map(|s| (*s).to_owned()).collect();
        import_edit(
            language,
            &requires,
            scan,
            &LineIndex::new(text),
            text,
            PositionEncoding::Utf8,
            Position::new(main_line, 4),
        )
    }

    #[test]
    fn missing_import_inserts_one_line_edit_at_the_offset() {
        let text = "use std::fs;\n\nfn main() {\n    \n}\n";
        let scan = ImportScan {
            statements: vec!["use std::fs;".to_owned()],
            insertion_offset: text.find("\nfn main").expect("fixture"),
            grouped_insertion_offset: None,
            leading_blank: false,
        };
        let edit =
            edit("rust", &["std::collections::HashMap"], &scan, text, 3).expect("edit expected");
        assert_eq!(edit.new_text, "use std::collections::HashMap;\n");
        assert_eq!(edit.range.start, Position::new(1, 0));
        assert_eq!(edit.range.start, edit.range.end, "pure insertion");
    }

    #[test]
    fn satisfied_requirement_yields_no_edit() {
        let text = "use std::collections::{HashMap, HashSet};\n\nfn main() {}\n";
        let scan = ImportScan {
            statements: vec!["use std::collections::{HashMap, HashSet};".to_owned()],
            insertion_offset: 43,
            grouped_insertion_offset: None,
            leading_blank: false,
        };
        assert!(edit("rust", &["std::collections::HashMap"], &scan, text, 2).is_none());
    }

    #[test]
    fn multiple_missing_lines_join_one_edit() {
        let text = "fn main() {\n    \n}\n";
        let scan = ImportScan::default();
        let edit = edit(
            "rust",
            &["std::collections::HashMap", "std::fmt::Write"],
            &scan,
            text,
            1,
        )
        .expect("edit expected");
        assert_eq!(
            edit.new_text,
            "use std::collections::HashMap;\nuse std::fmt::Write;\n"
        );
    }

    #[test]
    fn go_joins_the_grouped_block() {
        let text = "package main\n\nimport (\n\t\"fmt\"\n)\n\nfunc main() {\n\t\n}\n";
        let close = text.find(")\n").expect("fixture");
        let scan = ImportScan {
            statements: vec!["import (\n\t\"fmt\"\n)".to_owned()],
            insertion_offset: text.find("\nfunc main").expect("fixture"),
            grouped_insertion_offset: Some(close),
            leading_blank: false,
        };
        let edit = edit("go", &["errors"], &scan, text, 7).expect("edit expected");
        assert_eq!(edit.new_text, "\t\"errors\"\n");
        assert_eq!(
            edit.range.start,
            LineIndex::new(text).position(text, close, PositionEncoding::Utf8)
        );
    }

    #[test]
    fn go_zero_imports_gets_a_leading_blank_line() {
        let text = "package main\n\nfunc main() {\n\t\n}\n";
        let scan = ImportScan {
            statements: Vec::new(),
            insertion_offset: text.find("\nfunc main").expect("fixture"),
            grouped_insertion_offset: None,
            leading_blank: true,
        };
        let edit = edit("go", &["fmt"], &scan, text, 3).expect("edit expected");
        assert_eq!(edit.new_text, "\nimport \"fmt\"\n");
    }

    #[test]
    fn same_line_collision_drops_the_edit() {
        // A near-empty file: insertion line 0 == the expansion's line 0.
        let text = "hmap";
        let scan = ImportScan::default();
        assert!(edit("rust", &["std::collections::HashMap"], &scan, text, 0).is_none());
    }

    #[test]
    fn eof_without_trailing_newline_opens_a_line_first() {
        let text = "#pragma once\nint x;";
        let scan = ImportScan {
            statements: Vec::new(),
            insertion_offset: text.len(),
            grouped_insertion_offset: None,
            leading_blank: false,
        };
        let edit = edit("c", &["<stdio.h>"], &scan, text, 0).expect("edit expected");
        assert_eq!(edit.new_text, "\n#include <stdio.h>\n");
    }

    #[test]
    fn unsupported_language_specs_yield_no_edit() {
        let scan = ImportScan::default();
        assert!(edit("typescript", &["node:fs"], &scan, "let x;\n\n\n", 2).is_none());
    }
}
