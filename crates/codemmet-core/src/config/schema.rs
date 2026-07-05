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
    /// The template body. May be omitted only when `disabled = true`.
    #[serde(default)]
    pub body: Option<String>,
    /// Normalized scope that `${child}` content lives in (e.g. `"fn-body"`).
    /// Absent means the snippet is scope-transparent to nested children.
    #[serde(default)]
    pub scope: Option<String>,
    /// Context-conditional bodies (M3); first matching variant wins.
    #[serde(default)]
    pub variants: Vec<VariantDef>,
    /// In a user override file: remove this builtin trigger entirely.
    #[serde(default)]
    pub disabled: bool,
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
}
