//! The registry: builtin packs merged with user overrides, per language.

use std::collections::HashMap;
use std::path::Path;

use crate::config::loader::{ConfigError, SnippetSet};
use crate::lang::profile::LanguageProfile;

/// One language's profile plus its merged snippet set.
#[derive(Debug)]
pub struct LanguageSnippets {
    /// The static language profile.
    pub profile: &'static LanguageProfile,
    /// Builtin snippets merged with any user overrides.
    pub set: SnippetSet,
}

/// All loaded languages, ready for lookup by id, extension, or LSP id.
#[derive(Debug)]
pub struct Registry {
    languages: HashMap<&'static str, LanguageSnippets>,
}

impl Registry {
    /// Load builtin packs only (no user overrides). Used by tests.
    pub fn with_builtins() -> Result<Self, ConfigError> {
        Self::with_user_dir(None)
    }

    /// Load builtin packs, then merge `<dir>/<lang>.toml` user overrides.
    ///
    /// A missing directory or missing per-language file is not an error;
    /// an unreadable or invalid file is.
    pub fn with_user_dir(dir: Option<&Path>) -> Result<Self, ConfigError> {
        let _ = dir;
        todo!("M1: compile builtin packs, merge user override files")
    }

    /// Look up a language by id, file extension, or LSP language id.
    pub fn language(&self, id_or_ext: &str) -> Option<&LanguageSnippets> {
        crate::lang::profile::find(id_or_ext).and_then(|p| self.languages.get(p.id))
    }

    /// All loaded languages, in unspecified order.
    pub fn languages(&self) -> impl Iterator<Item = &LanguageSnippets> {
        self.languages.values()
    }
}
