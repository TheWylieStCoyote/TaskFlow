//! Per-language import requirement specs (`requires = [...]`).
//!
//! A snippet declares what it needs as a small, language-specific **spec**
//! string (see `IMPORT_REQUIRES_PLAN.md`); this module parses a spec into an
//! [`ImportSpec`] that can render the import line to insert and decide
//! whether an existing import statement already satisfies it.
//!
//! Spec forms per language:
//!
//! | language | spec                        | rendered line                        |
//! | -------- | --------------------------- | ------------------------------------ |
//! | rust     | `std::collections::HashMap` | `use std::collections::HashMap;`     |
//! | go       | `fmt`, `net/http`           | `import "fmt"`                       |
//! | python   | `json`                      | `import json`                        |
//! | python   | `dataclasses:dataclass`     | `from dataclasses import dataclass`  |
//! | c / cpp  | `<stdio.h>`, `"local.h"`    | `#include <stdio.h>`                 |
//!
//! Dedup is deliberately conservative in the *safe* direction: a false
//! negative inserts a harmless, visible duplicate, while a false positive
//! would silently skip a needed import — so [`ImportSpec::satisfied_by`]
//! only claims "already present" on a confident token match.

/// What the spec means, parsed per language.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// Rust `use` path; `symbol` is the final segment the code refers to.
    RustUse {
        /// Final path segment (`HashMap` of `std::collections::HashMap`).
        symbol: String,
    },
    /// Go package import path.
    GoImport {
        /// The import path, unquoted (`fmt`, `net/http`).
        path: String,
    },
    /// Python `import module`.
    PyImport {
        /// The (possibly dotted) module name.
        module: String,
    },
    /// Python `from module import name`.
    PyFrom {
        /// The (possibly dotted) module name.
        module: String,
        /// The imported name.
        name: String,
    },
    /// C/C++ `#include` with its delimiters (`<stdio.h>` or `"local.h"`).
    CInclude {
        /// The header including its `<>`/`""` delimiters.
        header: String,
    },
}

/// A parsed, language-validated import requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSpec {
    /// The raw spec string as written in the pack.
    spec: String,
    kind: Kind,
}

impl ImportSpec {
    /// The raw spec string as written in the pack.
    pub fn spec(&self) -> &str {
        &self.spec
    }

    /// The import line to insert (no trailing newline).
    pub fn line(&self) -> String {
        match &self.kind {
            Kind::RustUse { .. } => format!("use {};", self.spec),
            Kind::GoImport { path } => format!("import \"{path}\""),
            Kind::PyImport { module } => format!("import {module}"),
            Kind::PyFrom { module, name } => format!("from {module} import {name}"),
            Kind::CInclude { header } => format!("#include {header}"),
        }
    }

    /// The line to insert *inside* an existing Go grouped `import ( … )`
    /// block, or `None` for every other kind.
    pub fn grouped_line(&self) -> Option<String> {
        match &self.kind {
            Kind::GoImport { path } => Some(format!("\t\"{path}\"")),
            _ => None,
        }
    }

    /// Whether the text of one existing import statement already provides
    /// this requirement.
    ///
    /// Token-boundary matching, biased so that only confident matches count
    /// as "present" (an uncertain case inserts a visible duplicate instead
    /// of silently skipping a needed import).
    pub fn satisfied_by(&self, statement: &str) -> bool {
        match &self.kind {
            // `use std::collections::{HashMap, HashSet};` provides `HashMap`.
            // Known accepted imprecision (documented): `use x::HashMap as Y;`
            // also matches on the token.
            Kind::RustUse { symbol } => contains_token(statement, symbol),
            // The quoted path is unambiguous inside an import declaration.
            Kind::GoImport { path } => statement.contains(&format!("\"{path}\"")),
            // Only an `import …` statement binds the module name itself;
            // `from module import x` does not.
            Kind::PyImport { module } => {
                let statement = statement.trim_start();
                statement.starts_with("import") && contains_token(statement, module)
            }
            // Only `from module import … name …` binds the bare name.
            Kind::PyFrom { module, name } => {
                let statement = statement.trim_start();
                statement.starts_with("from")
                    && contains_token(statement, module)
                    && contains_token(statement, name)
            }
            Kind::CInclude { header } => statement.contains(header.as_str()),
        }
    }
}

/// Whether requires formatting is implemented for `language`.
///
/// `language` is a canonical profile id. JavaScript/TypeScript are deferred;
/// their packs may still *declare* requires, which `codemmet check` warns
/// about.
pub fn supports(language: &str) -> bool {
    matches!(language, "rust" | "go" | "python" | "c" | "cpp")
}

/// Parse and validate one requires spec for `language`.
///
/// # Errors
/// A human-readable message describing the violated spec form.
pub fn parse_spec(language: &str, spec: &str) -> Result<ImportSpec, String> {
    if spec.chars().any(char::is_control) {
        return Err("spec must not contain control characters".to_owned());
    }
    let kind = match language {
        "rust" => parse_rust(spec)?,
        "go" => parse_go(spec)?,
        "python" => parse_python(spec)?,
        "c" | "cpp" => parse_c(spec)?,
        other => return Err(format!("requires is not supported for `{other}`")),
    };
    Ok(ImportSpec {
        spec: spec.to_owned(),
        kind,
    })
}

/// Rust: a `::`-separated path of identifiers; the final segment is the
/// symbol the snippet body refers to.
fn parse_rust(spec: &str) -> Result<Kind, String> {
    if spec.starts_with("use ") || spec.ends_with(';') {
        return Err(
            "write the path only, e.g. `std::collections::HashMap` (not a `use` line)".to_owned(),
        );
    }
    let segments: Vec<&str> = spec.split("::").collect();
    if segments.iter().any(|s| !is_ident(s)) {
        return Err(format!(
            "expected a `::`-separated path of identifiers, got `{spec}`"
        ));
    }
    let symbol = (*segments.last().expect("split yields at least one")).to_owned();
    Ok(Kind::RustUse { symbol })
}

/// Go: a bare import path (`fmt`, `net/http`, `github.com/x/y`).
fn parse_go(spec: &str) -> Result<Kind, String> {
    let valid = !spec.is_empty()
        && !spec.starts_with('/')
        && !spec.ends_with('/')
        && spec
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'));
    if !valid {
        return Err(format!(
            "expected an import path like `net/http`, got `{spec}`"
        ));
    }
    Ok(Kind::GoImport {
        path: spec.to_owned(),
    })
}

/// Python: `module` (dotted) for `import module`, or `module:name` for
/// `from module import name`.
fn parse_python(spec: &str) -> Result<Kind, String> {
    let (module, name) = match spec.split_once(':') {
        Some((module, name)) => (module, Some(name)),
        None => (spec, None),
    };
    if !module.split('.').all(is_ident) {
        return Err(format!(
            "expected `module` or `module:name` with dotted-identifier module, got `{spec}`"
        ));
    }
    match name {
        Some(name) if !is_ident(name) => Err(format!(
            "expected an identifier after `:`, got `{name}` in `{spec}`"
        )),
        Some(name) => Ok(Kind::PyFrom {
            module: module.to_owned(),
            name: name.to_owned(),
        }),
        None => Ok(Kind::PyImport {
            module: module.to_owned(),
        }),
    }
}

/// C/C++: the header **with** its delimiters — `<stdio.h>` (system) or
/// `"local.h"` (local); the delimiters disambiguate the include form.
fn parse_c(spec: &str) -> Result<Kind, String> {
    let inner = spec
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .or_else(|| spec.strip_prefix('"').and_then(|s| s.strip_suffix('"')));
    let valid = inner.is_some_and(|inner| {
        !inner.is_empty()
            && inner
                .chars()
                .all(|c| !c.is_whitespace() && !matches!(c, '<' | '>' | '"'))
    });
    if !valid {
        return Err(format!(
            "expected `<header.h>` or `\"header.h\"` (delimiters required), got `{spec}`"
        ));
    }
    Ok(Kind::CInclude {
        header: spec.to_owned(),
    })
}

/// Whether `s` is a plain identifier: `[A-Za-z_][A-Za-z0-9_]*`.
fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `needle` occurs in `hay` at identifier-token boundaries (the
/// characters before and after the match are not `[A-Za-z0-9_]`).
fn contains_token(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut from = 0;
    while let Some(pos) = hay[from..].find(needle) {
        let start = from + pos;
        let end = start + needle.len();
        let boundary_before = hay[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word_char(c));
        let boundary_after = hay[end..].chars().next().is_none_or(|c| !is_word_char(c));
        if boundary_before && boundary_after {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Whether `c` is an identifier character.
const fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_spec_renders_use_line_and_symbol_key() {
        let spec = parse_spec("rust", "std::collections::HashMap").expect("valid");
        assert_eq!(spec.line(), "use std::collections::HashMap;");
        assert!(spec.satisfied_by("use std::collections::HashMap;"));
        assert!(spec.satisfied_by("use std::collections::{HashMap, HashSet};"));
        // Token boundary: `HashMapExt` is not `HashMap`.
        assert!(!spec.satisfied_by("use other::HashMapExt;"));
        // A glob does not name the symbol: conservative "not present".
        assert!(!spec.satisfied_by("use std::collections::*;"));
        assert!(spec.grouped_line().is_none());
    }

    #[test]
    fn rust_spec_rejects_use_lines_and_bad_paths() {
        assert!(parse_spec("rust", "use std::fmt;").is_err());
        assert!(parse_spec("rust", "std::fmt;").is_err());
        assert!(parse_spec("rust", "std::").is_err());
        assert!(parse_spec("rust", "std::collections::*").is_err());
        assert!(parse_spec("rust", "").is_err());
    }

    #[test]
    fn go_spec_renders_quoted_import_and_grouped_line() {
        let spec = parse_spec("go", "net/http").expect("valid");
        assert_eq!(spec.line(), "import \"net/http\"");
        assert_eq!(spec.grouped_line().as_deref(), Some("\t\"net/http\""));
        assert!(spec.satisfied_by("import \"net/http\""));
        assert!(spec.satisfied_by("import (\n\t\"fmt\"\n\t\"net/http\"\n)"));
        assert!(!spec.satisfied_by("import \"net/http/httptest\""));
    }

    #[test]
    fn go_spec_rejects_malformed_paths() {
        assert!(parse_spec("go", "\"fmt\"").is_err());
        assert!(parse_spec("go", "/fmt").is_err());
        assert!(parse_spec("go", "fmt ").is_err());
        assert!(parse_spec("go", "").is_err());
    }

    #[test]
    fn python_module_spec_matches_only_import_statements() {
        let spec = parse_spec("python", "json").expect("valid");
        assert_eq!(spec.line(), "import json");
        assert!(spec.satisfied_by("import json"));
        assert!(spec.satisfied_by("import os, json"));
        // `from json import loads` does not bind the bare `json` name.
        assert!(!spec.satisfied_by("from json import loads"));
    }

    #[test]
    fn python_from_spec_matches_only_from_imports() {
        let spec = parse_spec("python", "dataclasses:dataclass").expect("valid");
        assert_eq!(spec.line(), "from dataclasses import dataclass");
        assert!(spec.satisfied_by("from dataclasses import dataclass"));
        assert!(spec.satisfied_by("from dataclasses import field, dataclass"));
        // `import dataclasses` does not bind the bare `dataclass` name,
        // and `dataclasses` does not token-match `dataclass`.
        assert!(!spec.satisfied_by("import dataclasses"));
        assert!(!spec.satisfied_by("from dataclasses import field"));
    }

    #[test]
    fn python_spec_rejects_malformed_forms() {
        assert!(parse_spec("python", "1module").is_err());
        assert!(parse_spec("python", "m:not ident").is_err());
        assert!(parse_spec("python", "m:").is_err());
        assert!(parse_spec("python", "").is_err());
        // Dotted modules are fine.
        assert!(parse_spec("python", "os.path").is_ok());
    }

    #[test]
    fn c_spec_requires_delimiters_and_matches_exactly() {
        let spec = parse_spec("c", "<stdio.h>").expect("valid");
        assert_eq!(spec.line(), "#include <stdio.h>");
        assert!(spec.satisfied_by("#include <stdio.h>"));
        assert!(!spec.satisfied_by("#include \"stdio.h\""));
        let local = parse_spec("cpp", "\"local.h\"").expect("valid");
        assert_eq!(local.line(), "#include \"local.h\"");
    }

    #[test]
    fn c_spec_rejects_bare_and_malformed_headers() {
        assert!(parse_spec("c", "stdio.h").is_err());
        assert!(parse_spec("c", "<stdio.h").is_err());
        assert!(parse_spec("c", "<>").is_err());
        assert!(parse_spec("c", "<std io.h>").is_err());
    }

    #[test]
    fn unsupported_languages_are_rejected_and_reported() {
        assert!(!supports("javascript"));
        assert!(!supports("typescript"));
        assert!(supports("rust") && supports("cpp"));
        assert!(parse_spec("javascript", "fs").is_err());
    }

    #[test]
    fn control_characters_are_rejected_everywhere() {
        assert!(parse_spec("rust", "std::a\nuse evil::X").is_err());
        assert!(parse_spec("c", "<a\nb>").is_err());
    }
}
