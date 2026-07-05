//! Core abbreviation-expansion engine for codemmet.
//!
//! This crate owns the abbreviation mini-grammar, the snippet intermediate
//! representation, the per-language renderer, and the TOML snippet loader.
//! It deliberately has no tree-sitter, async-runtime, or CLI dependencies;
//! editor context arrives through the [`context::ContextProvider`] trait.

pub mod context;
