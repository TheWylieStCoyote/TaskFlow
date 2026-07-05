//! Composition and rendering: abbreviation AST + compiled snippets → text.

use crate::abbr::Abbreviation;
use crate::config::loader::SnippetSet;
use crate::context::ExpansionContext;

/// How tabstops appear in the rendered output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabstopMode {
    /// Emit LSP snippet syntax (`${1:placeholder}`, `$0`).
    Keep,
    /// Emit plain text: placeholders become their text, markers vanish.
    Strip,
}

/// Cursor position within rendered plain text (0-based line/character).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPos {
    /// 0-based line within the expansion body.
    pub line: u32,
    /// 0-based UTF-8 byte column within that line.
    pub character: u32,
}

/// A rendered expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expansion {
    /// The expansion text. LSP snippet syntax when `is_snippet` is true.
    pub body: String,
    /// Whether `body` uses LSP snippet syntax ([`TabstopMode::Keep`]).
    pub is_snippet: bool,
    /// Where the cursor lands, for [`TabstopMode::Strip`] output.
    pub cursor: Option<CursorPos>,
}

/// A failure while resolving or composing an abbreviation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// A WORD did not resolve to any registered trigger.
    #[error("unknown trigger `{word}`{}", suggestion.as_deref().map(|s| format!(" (did you mean `{s}`?)")).unwrap_or_default())]
    UnknownTrigger {
        /// The unresolved WORD from the abbreviation.
        word: String,
        /// Closest registered trigger by edit distance, if any is close.
        suggestion: Option<String>,
    },
    /// Arguments could not be bound to the snippet's parameters.
    #[error("cannot bind arguments for `{trigger}`: {message}")]
    ArgBinding {
        /// The trigger whose arguments failed to bind.
        trigger: String,
        /// What went wrong.
        message: String,
    },
}

/// Render a parsed abbreviation against a language's snippet set.
pub fn render(
    abbr: &Abbreviation,
    set: &SnippetSet,
    ctx: &ExpansionContext,
    mode: TabstopMode,
) -> Result<Expansion, RenderError> {
    let _ = (abbr, set, ctx, mode);
    todo!("M1: composer/renderer with global tabstop renumbering")
}
