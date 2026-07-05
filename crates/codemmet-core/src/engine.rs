//! The façade tying parser, registry, and renderer together.

use crate::abbr::ParseError;
use crate::config::loader::{default_user_snippets_dir, ConfigError};
use crate::context::ExpansionContext;
use crate::lang::registry::Registry;
use crate::snippet::render::{render, Expansion, RenderError, TabstopMode};

/// Why an expansion request failed.
#[derive(Debug, thiserror::Error)]
pub enum ExpandError {
    /// The requested language is not registered.
    #[error("unknown language `{0}`")]
    UnknownLanguage(String),
    /// The abbreviation text did not parse.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// The abbreviation parsed but could not be resolved or composed.
    #[error(transparent)]
    Render(#[from] RenderError),
}

/// The expansion engine: a loaded [`Registry`] plus the render pipeline.
#[derive(Debug)]
pub struct Engine {
    registry: Registry,
}

impl Engine {
    /// Engine with builtin packs merged with the default user override dir
    /// (`<config>/codemmet/snippets`).
    pub fn new() -> Result<Self, ConfigError> {
        let dir = default_user_snippets_dir();
        Ok(Self {
            registry: Registry::with_user_dir(dir.as_deref())?,
        })
    }

    /// Engine over an explicitly constructed registry.
    pub const fn with_registry(registry: Registry) -> Self {
        Self { registry }
    }

    /// Expand `abbr` for `language` (id, extension, or LSP language id).
    pub fn expand(
        &self,
        abbr: &str,
        language: &str,
        ctx: &ExpansionContext,
        mode: TabstopMode,
    ) -> Result<Expansion, ExpandError> {
        let lang = self
            .registry
            .language(language)
            .ok_or_else(|| ExpandError::UnknownLanguage(language.to_string()))?;
        let ast = crate::abbr::parse(abbr)?;
        Ok(render(&ast, &lang.set, ctx, mode)?)
    }

    /// The underlying registry, for listings and completion.
    pub const fn registry(&self) -> &Registry {
        &self.registry
    }
}
