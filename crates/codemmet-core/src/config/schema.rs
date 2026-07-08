//! Serde types mirroring the snippet-pack TOML schema (see `PLAN.md`).

use std::collections::BTreeMap;

use serde::Deserialize;

/// A whole snippet-pack file (`rust.toml`, `~/.config/codemmet/snippets/*.toml`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnippetFile {
    /// The `[meta]` table.
    pub meta: Meta,
    /// The `[snippets.<trigger>]` tables, keyed by trigger.
    #[serde(default)]
    pub snippets: BTreeMap<String, SnippetDef>,
}

/// The `[meta]` table of a snippet pack.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    /// Language identifier this pack targets, e.g. `"rust"`.
    pub language: String,
}

/// One `[snippets.<trigger>]` definition.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnippetDef {
    /// Short human-readable description shown in listings and completion.
    #[serde(default)]
    pub description: Option<String>,
    /// Named parameters, in binding order.
    #[serde(default)]
    pub params: Vec<ParamDef>,
    /// The template body. May be omitted only when `disabled = true` or when
    /// this is a composite (`expands` is set).
    #[serde(default)]
    pub body: Option<String>,
    /// Marks a **composite** (alias) snippet: its value is an *abbreviation*
    /// that is re-expanded, not a literal template body. Mutually exclusive
    /// with `body`. `${name}` references a declared parameter; the composite's
    /// selected options and freeform `>`-children merge into the expansion (see
    /// `COMPOSITE_SNIPPETS.md`).
    #[serde(default)]
    pub expands: Option<String>,
    /// Named option fragments a caller selects by name (`classp:Point,eq`).
    /// Each value is an abbreviation fragment merged into the composite's
    /// expansion as a sibling of the base pieces. Only valid with `expands`.
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    /// Normalized scope that `${child}` content lives in (e.g. `"fn-body"`).
    /// Absent means the snippet is scope-transparent to nested children.
    #[serde(default)]
    pub scope: Option<String>,
    /// Context-conditional bodies (M3); first matching variant wins.
    #[serde(default)]
    pub variants: Vec<VariantDef>,
    /// Import requirements: per-language specs (e.g. Rust
    /// `"std::collections::HashMap"`, C `"<stdio.h>"`, Python
    /// `"dataclasses:dataclass"`) whose imports editors insert alongside the
    /// expansion when missing. See `lang::imports` for the spec forms.
    #[serde(default)]
    pub requires: Vec<String>,
    /// A **multi-file** snippet: `[snippets.<trigger>.files.<label>]` sections,
    /// each targeting a file with its own `body`/`expands`. Mutually exclusive
    /// with a top-level `body`/`expands`. See `MULTIFILE_SNIPPETS.md`.
    #[serde(default)]
    pub files: BTreeMap<String, FileDef>,
    /// In a user override file: remove this builtin trigger entirely.
    #[serde(default)]
    pub disabled: bool,
}

/// One `[snippets.<trigger>.files.<label>]` section of a multi-file snippet.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileDef {
    /// Target path relative to the scaffold root, `${param}`-substituted
    /// (e.g. `"${app}/models.py"`). Mutually exclusive with `target`.
    #[serde(default)]
    pub path: Option<String>,
    /// Target derived from the invocation's current file: `"self"` (the current
    /// file) or a sibling by extension (e.g. `".cpp"`). Mutually exclusive with
    /// `path`.
    #[serde(default)]
    pub target: Option<String>,
    /// Template body for this file (LSP snippet syntax plus `${param}`).
    /// Mutually exclusive with `expands`.
    #[serde(default)]
    pub body: Option<String>,
    /// A composite-style abbreviation re-expanded for this file. Mutually
    /// exclusive with `body`.
    #[serde(default)]
    pub expands: Option<String>,
    /// What to do when the target file already exists (default: `inject`).
    #[serde(default)]
    pub on_exists: OnExists,
    /// For `on_exists = "inject"`: the name of the container (class/struct/…)
    /// to inject into, `${param}`-substituted. Absent injects into the last
    /// container in the file.
    #[serde(default)]
    pub into: Option<String>,
    /// Text prepended only when the file is *created* (e.g. `#pragma once`, an
    /// import block).
    #[serde(default)]
    pub on_create: Option<String>,
    /// This file receives the tabstops and final cursor. At most one per
    /// snippet; the others insert plain text (tabstops cannot cross files).
    #[serde(default)]
    pub primary: bool,
}

/// What a multi-file section does when its target already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnExists {
    /// Inject into the right structural place (Phase 2); until the injector
    /// lands this degrades to [`Self::Append`].
    #[default]
    Inject,
    /// Append the content to the end of the file.
    Append,
    /// Only create; treat an existing target as an error.
    Create,
}

/// One entry of a snippet's `params` list.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamDef {
    /// The name referenced as `${name}` in the body.
    pub name: String,
    /// How glued abbreviation args bind to this param.
    #[serde(default)]
    pub kind: ParamKind,
    /// Placeholder text when the abbreviation supplies no argument.
    #[serde(default)]
    pub default: Option<String>,
}

/// Argument-binding kind for a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamKind {
    /// Binds identifier-like glued args (e.g. the `x` of `forx`).
    Ident,
    /// Binds numeric glued args (e.g. the `10` of `fori10`).
    Int,
    /// Only binds explicit `:`-args.
    #[default]
    Str,
}

/// A `[[snippets.<trigger>.variants]]` entry (context-aware body, M3).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantDef {
    /// Normalized scope name (e.g. `"type-body"`) or raw node kind as `"ts:<kind>"`.
    pub context: String,
    /// The variant's template body.
    pub body: String,
    /// Import requirements for this variant. When the variant is selected and
    /// this is declared (even as `[]`), it **replaces** the snippet-level
    /// `requires` — mirroring how the variant's body replaces the default
    /// body. Absent means inherit the snippet-level list.
    #[serde(default)]
    pub requires: Option<Vec<String>>,
}
