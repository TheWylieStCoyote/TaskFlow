//! Static descriptions of each supported language.

use crate::context::IndentUnit;

/// Compile-time facts about one supported language.
#[derive(Debug, Clone)]
pub struct LanguageProfile {
    /// Canonical identifier, e.g. `"rust"`.
    pub id: &'static str,
    /// LSP `languageId` values that map to this profile.
    pub lsp_language_ids: &'static [&'static str],
    /// File extensions (without dot) that map to this profile.
    pub extensions: &'static [&'static str],
    /// Indentation used when the editor context supplies none.
    pub default_indent: IndentUnit,
    /// The builtin snippet pack, compiled into the binary.
    pub snippet_source: &'static str,
}

/// All builtin language profiles. Rust and Python ship in M1; C, C++,
/// JavaScript, TypeScript, and Go land in M4 (see `PLAN.md`).
pub static PROFILES: &[LanguageProfile] = &[
    LanguageProfile {
        id: "rust",
        lsp_language_ids: &["rust"],
        extensions: &["rs"],
        default_indent: IndentUnit::Spaces(4),
        snippet_source: include_str!("../../snippets/rust.toml"),
    },
    LanguageProfile {
        id: "python",
        lsp_language_ids: &["python"],
        extensions: &["py"],
        default_indent: IndentUnit::Spaces(4),
        snippet_source: include_str!("../../snippets/python.toml"),
    },
];

/// Find a profile by id, file extension, or LSP language id.
pub fn find(id_or_ext: &str) -> Option<&'static LanguageProfile> {
    PROFILES.iter().find(|p| {
        p.id == id_or_ext
            || p.extensions.contains(&id_or_ext)
            || p.lsp_language_ids.contains(&id_or_ext)
    })
}
