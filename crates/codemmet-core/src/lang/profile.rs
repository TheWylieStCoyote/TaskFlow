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
    /// Language id whose builtin pack this language inherits: that pack is
    /// compiled BEFORE this language's own pack, so this pack overrides and
    /// extends it (e.g. C++ inherits C, TypeScript inherits JavaScript).
    pub inherits: Option<&'static str>,
}

/// All builtin language profiles (see `PLAN.md`).
pub static PROFILES: &[LanguageProfile] = &[
    LanguageProfile {
        id: "rust",
        lsp_language_ids: &["rust"],
        extensions: &["rs"],
        default_indent: IndentUnit::Spaces(4),
        snippet_source: include_str!("../../snippets/rust.toml"),
        inherits: None,
    },
    LanguageProfile {
        id: "python",
        lsp_language_ids: &["python"],
        extensions: &["py"],
        default_indent: IndentUnit::Spaces(4),
        snippet_source: include_str!("../../snippets/python.toml"),
        inherits: None,
    },
    LanguageProfile {
        id: "c",
        lsp_language_ids: &["c"],
        extensions: &["c", "h"],
        default_indent: IndentUnit::Spaces(4),
        snippet_source: include_str!("../../snippets/c.toml"),
        inherits: None,
    },
    LanguageProfile {
        id: "cpp",
        lsp_language_ids: &["cpp"],
        extensions: &["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
        default_indent: IndentUnit::Spaces(4),
        snippet_source: include_str!("../../snippets/cpp.toml"),
        inherits: Some("c"),
    },
    LanguageProfile {
        id: "javascript",
        lsp_language_ids: &["javascript", "javascriptreact"],
        extensions: &["js", "mjs", "cjs", "jsx"],
        default_indent: IndentUnit::Spaces(2),
        snippet_source: include_str!("../../snippets/javascript.toml"),
        inherits: None,
    },
    LanguageProfile {
        id: "typescript",
        lsp_language_ids: &["typescript", "typescriptreact"],
        extensions: &["ts", "mts", "cts", "tsx"],
        default_indent: IndentUnit::Spaces(2),
        snippet_source: include_str!("../../snippets/typescript.toml"),
        inherits: Some("javascript"),
    },
    LanguageProfile {
        id: "go",
        lsp_language_ids: &["go"],
        extensions: &["go"],
        default_indent: IndentUnit::Tab,
        snippet_source: include_str!("../../snippets/go.toml"),
        inherits: None,
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
