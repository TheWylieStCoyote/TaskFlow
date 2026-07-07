//! Writing snippets back into a TOML pack file.
//!
//! The counterpart to [`loader`](crate::config::loader): where the loader
//! reads packs, this module inserts (or replaces) a single `[snippets.<name>]`
//! table in a pack, creating the file with a `[meta]` header when it does not
//! exist yet. It is the persistence half of the capture-from-example workflow
//! (see [`capture`](crate::snippet::capture)).
//!
//! The write is minimally invasive: other snippets, comments, and formatting
//! in an existing pack are preserved. Only the target trigger's tables (the
//! `[snippets.<name>]` table and any `[[snippets.<name>.variants]]` of it) are
//! removed before the new one is appended. The result is always re-validated
//! through the loader before it touches disk, so a pack this module writes is
//! guaranteed to pass `codemmet check`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::config::loader::{ConfigError, SnippetSet, parse_snippet_file};
use crate::config::schema::ParamDef;

/// One snippet to write into a pack.
#[derive(Debug, Clone)]
pub struct SnippetEntry {
    /// The trigger word.
    pub trigger: String,
    /// Optional human-readable description.
    pub description: Option<String>,
    /// Declared parameters, in binding order.
    pub params: Vec<ParamDef>,
    /// The template body (without the surrounding `'''`).
    pub body: String,
}

/// Why writing a snippet pack failed.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The pack file could not be read or written.
    #[error("failed to access {path}: {source}")]
    Io {
        /// The file that failed.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The trigger already exists and `force` was not set.
    #[error("trigger `{trigger}` already exists in {path} (use --force to overwrite)")]
    Exists {
        /// The conflicting trigger.
        trigger: String,
        /// The pack it is in.
        path: PathBuf,
    },
    /// The body cannot be represented as a `'''` literal string.
    #[error("snippet body contains `'''`, which cannot be written to a TOML pack")]
    UnrepresentableBody,
    /// The pack that would result is invalid (existing file is malformed, or
    /// the new snippet violates the schema).
    #[error(transparent)]
    Invalid(#[from] ConfigError),
}

/// Insert or replace `entry` in `<dir>/<lang>.toml`, creating the file (and
/// `dir`) if needed.
///
/// Returns the path written. If the trigger already exists and `force` is
/// false, fails with [`WriteError::Exists`] and leaves the file untouched.
pub fn upsert_snippet(
    dir: &Path,
    lang: &str,
    entry: &SnippetEntry,
    force: bool,
) -> Result<PathBuf, WriteError> {
    if entry.body.contains("'''") {
        return Err(WriteError::UnrepresentableBody);
    }
    let path = dir.join(format!("{lang}.toml"));

    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => return Err(WriteError::Io { path, source }),
    };

    // A trigger already present in the file conflicts unless we are forcing.
    if let Some(text) = &existing {
        let file = parse_snippet_file(&path.display().to_string(), text)?;
        if file.snippets.contains_key(&entry.trigger) && !force {
            return Err(WriteError::Exists {
                trigger: entry.trigger.clone(),
                path,
            });
        }
    }

    let base = existing.as_ref().map_or_else(
        || format!("[meta]\nlanguage = \"{lang}\"\n"),
        |text| strip_trigger(text, &entry.trigger),
    );
    let new_text = append_snippet(&base, entry);

    // Never write something the loader would reject.
    let file = parse_snippet_file(&path.display().to_string(), &new_text)?;
    SnippetSet::compile(&[(&path.display().to_string(), &file)])?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| WriteError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(&path, &new_text).map_err(|source| WriteError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Remove every table owned by `trigger` (its `[snippets.<trigger>]` and any
/// `[[snippets.<trigger>.variants]]`) from `text`, keeping everything else.
fn strip_trigger(text: &str, trigger: &str) -> String {
    let own = format!("snippets.{trigger}");
    let own_prefix = format!("snippets.{trigger}.");
    let mut out = String::with_capacity(text.len());
    let mut dropping = false;
    for line in text.split_inclusive('\n') {
        if let Some(path) = table_header_path(line.trim_end()) {
            dropping = path == own || path.starts_with(&own_prefix);
        }
        if !dropping {
            out.push_str(line);
        }
    }
    out
}

/// The dotted key path of a TOML table header line (`[a.b]` or `[[a.b]]`), or
/// `None` if the line is not a table header.
fn table_header_path(line: &str) -> Option<&str> {
    let line = line.trim();
    let inner = line
        .strip_prefix("[[")
        .and_then(|s| s.strip_suffix("]]"))
        .or_else(|| line.strip_prefix('[').and_then(|s| s.strip_suffix(']')))?;
    Some(inner.trim())
}

/// Append a rendered `[snippets.<trigger>]` block to `base`.
fn append_snippet(base: &str, entry: &SnippetEntry) -> String {
    let mut out = base.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    let _ = writeln!(out, "[snippets.{}]", entry.trigger);
    if let Some(description) = &entry.description {
        let _ = writeln!(out, "description = {}", toml_basic_string(description));
    }
    if !entry.params.is_empty() {
        out.push_str("params = [");
        for (i, param) in entry.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&render_param(param));
        }
        out.push_str("]\n");
    }
    out.push_str("body = '''\n");
    out.push_str(&entry.body);
    out.push_str("\n'''\n");
    out
}

/// Render one parameter as a TOML inline table.
fn render_param(param: &ParamDef) -> String {
    use crate::config::schema::ParamKind;
    let kind = match param.kind {
        ParamKind::Ident => "ident",
        ParamKind::Int => "int",
        ParamKind::Str => "str",
    };
    let mut out = format!(
        "{{ name = {}, kind = \"{kind}\"",
        toml_basic_string(&param.name)
    );
    if let Some(default) = &param.default {
        let _ = write!(out, ", default = {}", toml_basic_string(default));
    }
    out.push_str(" }");
    out
}

/// A TOML basic string (double-quoted) with the necessary escapes.
fn toml_basic_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::*;
    use crate::config::schema::ParamKind;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("codemmet-writer-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn entry(trigger: &str, body: &str, params: Vec<ParamDef>) -> SnippetEntry {
        SnippetEntry {
            trigger: trigger.to_owned(),
            description: Some("captured".to_owned()),
            params,
            body: body.to_owned(),
        }
    }

    fn ident(name: &str, default: &str) -> ParamDef {
        ParamDef {
            name: name.to_owned(),
            kind: ParamKind::Ident,
            default: Some(default.to_owned()),
        }
    }

    #[test]
    fn writes_a_new_pack_that_the_loader_accepts() {
        let dir = scratch("new-pack");
        let e = entry("mk", "fn make_${T}() -> ${T} {}", vec![ident("T", "T")]);
        let path = upsert_snippet(&dir, "rust", &e, false).expect("write should succeed");
        let text = std::fs::read_to_string(&path).expect("file should exist");
        assert!(text.contains("[meta]\nlanguage = \"rust\""));
        assert!(text.contains("[snippets.mk]"));
        assert!(text.contains("body = '''\nfn make_${T}() -> ${T} {}\n'''"));

        // The loader accepts it and the trigger compiles.
        let file = parse_snippet_file("t", &text).expect("parses");
        let set = SnippetSet::compile(&[("t", &file)]).expect("compiles");
        assert!(set.get("mk").is_some());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn existing_trigger_conflicts_without_force() {
        let dir = scratch("conflict");
        let e = entry("mk", "let ${x} = ${x};", vec![ident("x", "x")]);
        upsert_snippet(&dir, "rust", &e, false).expect("first write");
        let err = upsert_snippet(&dir, "rust", &e, false).expect_err("second should conflict");
        assert!(matches!(err, WriteError::Exists { .. }));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn force_replaces_the_trigger_and_keeps_siblings() {
        let dir = scratch("force");
        let path = dir.join("rust.toml");
        std::fs::write(
            &path,
            "[meta]\nlanguage = \"rust\"\n\n# keep me\n[snippets.other]\nbody = \"other()\"\n\n[snippets.mk]\nbody = \"old()\"\n",
        )
        .expect("seed pack");
        let e = entry("mk", "new()", vec![]);
        upsert_snippet(&dir, "rust", &e, true).expect("force write");
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains("# keep me"), "comment preserved: {text}");
        assert!(text.contains("[snippets.other]"), "sibling preserved");
        assert!(text.contains("body = '''\nnew()\n'''"), "replaced: {text}");
        assert!(!text.contains("old()"), "old body gone: {text}");
        // Exactly one `[snippets.mk]` table survives.
        assert_eq!(text.matches("[snippets.mk]").count(), 1, "text: {text}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn strip_trigger_removes_variant_subtables() {
        let text = "[meta]\nlanguage = \"rust\"\n[snippets.fn]\nbody = \"a\"\n[[snippets.fn.variants]]\ncontext = \"type-body\"\nbody = \"b\"\n[snippets.keep]\nbody = \"c\"\n";
        let stripped = strip_trigger(text, "fn");
        assert!(!stripped.contains("[snippets.fn]"));
        assert!(!stripped.contains("[[snippets.fn.variants]]"));
        assert!(stripped.contains("[snippets.keep]"));
    }
}
