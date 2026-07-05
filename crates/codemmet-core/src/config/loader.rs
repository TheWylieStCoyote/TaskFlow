//! Compiling snippet-pack files into validated, mergeable trigger sets.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::schema::{ParamDef, SnippetFile};
use crate::snippet::ir::Template;
use crate::snippet::template::TemplateError;

/// A snippet definition compiled and validated for rendering.
#[derive(Debug, Clone)]
pub struct CompiledSnippet {
    /// The trigger word.
    pub trigger: String,
    /// Short description for listings and completion.
    pub description: Option<String>,
    /// Named parameters in binding order.
    pub params: Vec<ParamDef>,
    /// The default (context-free) template.
    pub template: Template,
    /// Context-conditional templates (M3); first match wins.
    pub variants: Vec<CompiledVariant>,
}

/// A compiled context-conditional variant.
#[derive(Debug, Clone)]
pub struct CompiledVariant {
    /// Normalized scope name or `ts:`-prefixed raw node kind.
    pub context: String,
    /// The variant's template.
    pub template: Template,
}

/// A language's merged, validated triggers.
#[derive(Debug, Clone, Default)]
pub struct SnippetSet {
    snippets: BTreeMap<String, CompiledSnippet>,
}

/// A snippet pack that failed to load or validate.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A user snippet file could not be read.
    #[error("failed to read {path}: {source}")]
    Io {
        /// The file that failed to read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A pack is not valid TOML or violates the schema.
    #[error("invalid snippet pack {origin}: {source}")]
    Toml {
        /// Which pack (builtin name or file path) failed.
        origin: String,
        /// The TOML deserialization error.
        #[source]
        source: Box<toml::de::Error>,
    },
    /// A snippet body violates the template rules.
    #[error("invalid snippet `{trigger}` in {origin}: {source}")]
    Snippet {
        /// The offending trigger.
        trigger: String,
        /// Which pack it came from.
        origin: String,
        /// The template violation.
        #[source]
        source: TemplateError,
    },
    /// A schema rule that spans fields (e.g. missing body) was violated.
    #[error("invalid snippet `{trigger}` in {origin}: {message}")]
    Invalid {
        /// The offending trigger.
        trigger: String,
        /// Which pack it came from.
        origin: String,
        /// What rule was violated.
        message: String,
    },
}

impl SnippetSet {
    /// Compile packs in order; later packs override earlier ones per-trigger,
    /// and `disabled = true` entries remove the trigger.
    ///
    /// Each source is `(origin, file)` where `origin` names the pack for
    /// error messages (builtin name or file path).
    pub fn compile(sources: &[(&str, &SnippetFile)]) -> Result<Self, ConfigError> {
        let _ = sources;
        todo!("M1: validate bodies via parse_template, merge per-trigger")
    }

    /// Look up an exact trigger.
    pub fn get(&self, trigger: &str) -> Option<&CompiledSnippet> {
        self.snippets.get(trigger)
    }

    /// Split a WORD into the longest registered trigger prefix + remainder.
    pub fn resolve_word<'w>(&self, word: &'w str) -> Option<(&CompiledSnippet, &'w str)> {
        (1..=word.len()).rev().find_map(|len| {
            word.get(..len)
                .and_then(|prefix| self.get(prefix).map(|snippet| (snippet, &word[len..])))
        })
    }

    /// All triggers, in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = &CompiledSnippet> {
        self.snippets.values()
    }

    /// Number of registered triggers.
    pub fn len(&self) -> usize {
        self.snippets.len()
    }

    /// Whether the set has no triggers.
    pub fn is_empty(&self) -> bool {
        self.snippets.is_empty()
    }
}

/// Parse one pack's TOML text against the schema.
pub fn parse_snippet_file(origin: &str, text: &str) -> Result<SnippetFile, ConfigError> {
    let _ = (origin, text);
    todo!("M1: toml deserialize with origin-tagged errors")
}

/// The default user snippet directory: `<config>/codemmet/snippets`.
pub fn default_user_snippets_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("codemmet").join("snippets"))
}
