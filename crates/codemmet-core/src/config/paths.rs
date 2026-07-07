//! The personal path-prefix → snippet-directory mapping (`paths.toml`).
//!
//! This is how a snippet set that lives *outside* any repository can still
//! apply to chosen project paths without being committed. The user keeps a
//! `paths.toml` (by default `<config>/codemmet/paths.toml`) mapping a path
//! prefix to a snippet directory; whenever the file being edited is under that
//! prefix, the directory joins the override stack as a tier above the global
//! user directory (see
//! [`resolve_override_dirs`](crate::lang::registry::resolve_override_dirs)).
//!
//! ```toml
//! # ~/.config/codemmet/paths.toml
//! [[map]]
//! prefix = "~/work/acme"
//! dir    = "~/snippets/acme"
//!
//! [[map]]
//! prefix = "~/oss"
//! dir    = "~/snippets/rust-oss"
//! ```
//!
//! A longer (deeper) prefix is higher priority than a shorter one, so a more
//! specific mapping refines a broader one. `~` at the start of a `prefix` or
//! `dir` expands to the home directory.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::loader::ConfigError;

/// A loaded set of path-prefix → directory mappings.
#[derive(Debug, Clone, Default)]
pub struct PathMapping {
    /// Entries, in file order. `dirs_for` sorts matches by prefix depth.
    entries: Vec<MapEntry>,
}

/// One resolved mapping entry (paths already `~`-expanded).
#[derive(Debug, Clone)]
struct MapEntry {
    /// The absolute path prefix a file must be under to match.
    prefix: PathBuf,
    /// The snippet directory to add when the prefix matches.
    dir: PathBuf,
}

/// The `paths.toml` file shape.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PathsFile {
    /// The `[[map]]` entries.
    #[serde(default)]
    map: Vec<RawEntry>,
}

/// One raw `[[map]]` entry before `~` expansion.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEntry {
    /// Path prefix the file must be under.
    prefix: String,
    /// Snippet directory to apply for files under `prefix`.
    dir: String,
}

impl PathMapping {
    /// An empty mapping (no `paths.toml`, or none configured).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Whether the mapping has no entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Parse `paths.toml` text, expanding leading `~` in prefixes and dirs.
    pub fn parse(origin: &str, text: &str) -> Result<Self, ConfigError> {
        let file: PathsFile = toml::from_str(text).map_err(|source| ConfigError::Toml {
            origin: origin.to_owned(),
            source: Box::new(source),
        })?;
        let entries = file
            .map
            .into_iter()
            .map(|raw| MapEntry {
                prefix: expand_tilde(&raw.prefix),
                dir: expand_tilde(&raw.dir),
            })
            .collect();
        Ok(Self { entries })
    }

    /// Load `paths.toml` from `path`. A missing file is not an error — it
    /// yields an empty mapping; an unreadable or malformed one is.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&path.display().to_string(), &text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(source) => Err(ConfigError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    /// The snippet directories mapped to `file`, lowest priority first
    /// (shallow prefix → deep prefix).
    ///
    /// A prefix matches when it is a component-wise prefix of `file`, so
    /// `/a/b` matches `/a/b/c.rs` but not `/a/bc/d.rs`.
    #[must_use]
    pub fn dirs_for(&self, file: &Path) -> Vec<PathBuf> {
        let mut matches: Vec<&MapEntry> = self
            .entries
            .iter()
            .filter(|entry| file.starts_with(&entry.prefix))
            .collect();
        // Deeper (longer) prefixes are more specific → higher priority, so
        // they come last in the lowest-priority-first list.
        matches.sort_by_key(|entry| entry.prefix.components().count());
        matches.into_iter().map(|entry| entry.dir.clone()).collect()
    }
}

/// The default `paths.toml` location: `<config>/codemmet/paths.toml`.
#[cfg(feature = "dirs")]
#[must_use]
pub fn default_paths_config() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("codemmet").join("paths.toml"))
}

/// Without the `dirs` feature there is no platform directory to resolve.
#[cfg(not(feature = "dirs"))]
#[must_use]
pub const fn default_paths_config() -> Option<PathBuf> {
    None
}

/// Expand a leading `~` (or `~/…`) to the home directory; other paths pass
/// through unchanged. Without a resolvable home the tilde is left literal.
fn expand_tilde(s: &str) -> PathBuf {
    if s == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(s));
    }
    if let Some(rest) = s.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(s)
}

/// The home directory, when the `dirs` feature can resolve one.
#[cfg(feature = "dirs")]
fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// No home directory without the `dirs` feature.
#[cfg(not(feature = "dirs"))]
const fn home_dir() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mapping_matches_nothing() {
        assert!(
            PathMapping::empty()
                .dirs_for(Path::new("/any/where.rs"))
                .is_empty()
        );
    }

    #[test]
    fn matches_by_component_prefix_not_string_prefix() {
        let m = PathMapping::parse("t", "[[map]]\nprefix = \"/a/b\"\ndir = \"/snips/b\"\n")
            .expect("parses");
        assert_eq!(
            m.dirs_for(Path::new("/a/b/c/main.rs")),
            vec![PathBuf::from("/snips/b")]
        );
        // `/a/bc` must NOT match the `/a/b` prefix.
        assert!(m.dirs_for(Path::new("/a/bc/main.rs")).is_empty());
    }

    #[test]
    fn deeper_prefix_is_higher_priority_last() {
        let m = PathMapping::parse(
            "t",
            concat!(
                "[[map]]\nprefix = \"/work\"\ndir = \"/snips/work\"\n",
                "[[map]]\nprefix = \"/work/acme/svc\"\ndir = \"/snips/svc\"\n",
                "[[map]]\nprefix = \"/work/acme\"\ndir = \"/snips/acme\"\n",
            ),
        )
        .expect("parses");
        // All three match; ordered shallow → deep (lowest priority first).
        assert_eq!(
            m.dirs_for(Path::new("/work/acme/svc/src/main.rs")),
            vec![
                PathBuf::from("/snips/work"),
                PathBuf::from("/snips/acme"),
                PathBuf::from("/snips/svc"),
            ]
        );
    }

    #[test]
    fn unknown_key_is_rejected() {
        let err = PathMapping::parse("t", "[[map]]\nprefix = \"/a\"\ndir = \"/d\"\ncolor = 1\n")
            .expect_err("unknown key should be rejected");
        assert!(matches!(err, ConfigError::Toml { .. }));
    }

    #[test]
    fn missing_file_is_empty() {
        let m = PathMapping::load(Path::new("/no/such/paths.toml")).expect("missing is fine");
        assert!(m.is_empty());
    }
}
