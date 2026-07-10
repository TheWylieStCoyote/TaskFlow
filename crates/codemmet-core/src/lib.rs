//! Core abbreviation-expansion engine for codemmet.
//!
//! This crate owns the abbreviation mini-grammar, the snippet intermediate
//! representation, the per-language renderer, and the TOML snippet loader.
//! It deliberately has no tree-sitter, async-runtime, or CLI dependencies;
//! editor context arrives through the [`context::ContextProvider`] trait.
//!
//! Pipeline: abbreviation text → [`abbr::Abbreviation`] AST → per-trigger
//! [`snippet::ir::Template`]s composed into one IR → rendered
//! [`snippet::render::Expansion`] with LSP-style tabstops.

pub mod abbr;
pub mod config;
pub mod context;
pub mod engine;
pub mod filefmt;
pub mod lang;
pub mod snippet;

pub use engine::Engine;
