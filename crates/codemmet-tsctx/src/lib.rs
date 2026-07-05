//! Tree-sitter-backed [`ContextProvider`] for codemmet.
//!
//! This crate will own grammar loading, per-document tree caching, scope
//! classification, wrap-selection snapping, and tree-derived indentation
//! (milestones M3/M4 in `PLAN.md`). Until then it only re-exports the
//! provider seam so the binary can wire either implementation.
//!
//! [`ContextProvider`]: codemmet_core::context::ContextProvider

pub use codemmet_core::context::ContextProvider;

/// The tree-sitter ABI version this crate is built against.
///
/// Grammar crates added in M3 must be compatible with this version.
pub const TREE_SITTER_ABI_VERSION: usize = tree_sitter::LANGUAGE_VERSION;

#[cfg(test)]
mod tests {
    #[test]
    fn links_against_tree_sitter() {
        let parser = tree_sitter::Parser::new();
        assert!(parser.language().is_none());
    }
}
