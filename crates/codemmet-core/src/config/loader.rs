//! Compiling snippet-pack files into validated, mergeable trigger sets.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::config::schema::{ParamDef, SnippetDef, SnippetFile};
use crate::context::Scope;
use crate::snippet::ir::{Span, Template};
use crate::snippet::template::{TemplateError, parse_template};

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
    /// Normalized scope name that `${child}` content lives in, if declared.
    /// `None` means the snippet is scope-transparent: nested children keep
    /// the scope this snippet itself expanded in.
    pub scope: Option<String>,
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
        let mut snippets = BTreeMap::new();
        for &(origin, file) in sources {
            for (trigger, def) in &file.snippets {
                if !is_word(trigger) {
                    return Err(ConfigError::Invalid {
                        trigger: trigger.clone(),
                        origin: origin.to_owned(),
                        message: "trigger must be a word ([A-Za-z_][A-Za-z0-9_]*)".to_owned(),
                    });
                }
                if def.disabled {
                    snippets.remove(trigger);
                    continue;
                }
                let compiled = compile_snippet(origin, trigger, def)?;
                snippets.insert(trigger.clone(), compiled);
            }
        }
        Ok(Self { snippets })
    }

    /// Build a set directly from compiled snippets (fixtures, tests).
    pub fn from_compiled(snippets: impl IntoIterator<Item = CompiledSnippet>) -> Self {
        Self {
            snippets: snippets
                .into_iter()
                .map(|s| (s.trigger.clone(), s))
                .collect(),
        }
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
    toml::from_str(text).map_err(|source| ConfigError::Toml {
        origin: origin.to_owned(),
        source: Box::new(source),
    })
}

/// Parse a normalized scope name (e.g. `"type-body"`) into a [`Scope`].
///
/// These are the names the TOML schema accepts for `scope` fields and
/// variant `context`s; [`Scope::Unknown`] has no name and never parses.
pub fn parse_scope_name(name: &str) -> Option<Scope> {
    Some(match name {
        "root" => Scope::Root,
        "type-body" => Scope::TypeBody,
        "fn-body" => Scope::FnBody,
        "loop" => Scope::Loop,
        "conditional" => Scope::Conditional,
        "match-body" => Scope::MatchBody,
        "try-body" => Scope::TryBody,
        _ => return None,
    })
}

/// The canonical TOML-schema name of a [`Scope`].
pub const fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::Root => "root",
        Scope::TypeBody => "type-body",
        Scope::FnBody => "fn-body",
        Scope::Loop => "loop",
        Scope::Conditional => "conditional",
        Scope::MatchBody => "match-body",
        Scope::TryBody => "try-body",
        Scope::Unknown => "unknown",
    }
}

/// Whether `s` is a trigger WORD: `[A-Za-z_][A-Za-z0-9_]*`.
fn is_word(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The parameter names a template references, in span order (with repeats).
fn referenced_params(template: &Template) -> impl Iterator<Item = &str> {
    template
        .lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .filter_map(|span| match span {
            Span::Param { name } => Some(name.as_str()),
            _ => None,
        })
}

/// Validate and compile one non-disabled snippet definition.
fn compile_snippet(
    origin: &str,
    trigger: &str,
    def: &SnippetDef,
) -> Result<CompiledSnippet, ConfigError> {
    let invalid = |message: String| ConfigError::Invalid {
        trigger: trigger.to_owned(),
        origin: origin.to_owned(),
        message,
    };
    let snippet_err = |source: TemplateError| ConfigError::Snippet {
        trigger: trigger.to_owned(),
        origin: origin.to_owned(),
        source,
    };

    let body = def
        .body
        .as_deref()
        .ok_or_else(|| invalid("missing body".to_owned()))?;
    let template = parse_template(body).map_err(snippet_err)?;

    if let Some(scope) = &def.scope
        && parse_scope_name(scope).is_none()
    {
        return Err(invalid(format!(
            "unknown scope `{scope}` (expected one of root, type-body, \
             fn-body, loop, conditional, match-body, try-body)"
        )));
    }

    let mut variants = Vec::with_capacity(def.variants.len());
    for variant in &def.variants {
        let is_ts = variant.context.starts_with("ts:");
        if !is_ts && parse_scope_name(&variant.context).is_none() {
            return Err(invalid(format!(
                "unknown variant context `{}` (expected a scope name or \
                 `ts:<node-kind>`)",
                variant.context
            )));
        }
        variants.push(CompiledVariant {
            context: variant.context.clone(),
            template: parse_template(&variant.body).map_err(snippet_err)?,
        });
    }

    let mut declared = BTreeSet::new();
    for param in &def.params {
        if param.name == "i" {
            return Err(invalid(
                "parameter name `i` is reserved for the repetition index `${i}`".to_owned(),
            ));
        }
        if param.name == "value" {
            return Err(invalid(
                "parameter name `value` is reserved for the data-driven repeat item `${value}`"
                    .to_owned(),
            ));
        }
        if !declared.insert(param.name.as_str()) {
            return Err(invalid(format!("duplicate parameter `{}`", param.name)));
        }
    }

    // Every `${name}` reference (default body and variants) must be declared.
    for referenced in std::iter::once(&template)
        .chain(variants.iter().map(|v| &v.template))
        .flat_map(referenced_params)
    {
        if !declared.contains(referenced) {
            return Err(invalid(format!(
                "body references undeclared parameter `{referenced}`"
            )));
        }
    }

    // Every declared param must appear in the default body; variants may
    // omit params.
    let default_refs: BTreeSet<&str> = referenced_params(&template).collect();
    for param in &def.params {
        if !default_refs.contains(param.name.as_str()) {
            return Err(invalid(format!(
                "parameter `{}` is never referenced in the body",
                param.name
            )));
        }
    }

    Ok(CompiledSnippet {
        trigger: trigger.to_owned(),
        description: def.description.clone(),
        params: def.params.clone(),
        template,
        scope: def.scope.clone(),
        variants,
    })
}

/// The default user snippet directory: `<config>/codemmet/snippets`.
///
/// Requires the `dirs` feature (on by default); absent it — e.g. in a WASM
/// build that has no home directory — this returns `None`.
#[cfg(feature = "dirs")]
#[must_use]
pub fn default_user_snippets_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("codemmet").join("snippets"))
}

/// The default user snippet directory. Without the `dirs` feature there is no
/// platform directory to resolve, so this is always `None`.
#[cfg(not(feature = "dirs"))]
#[must_use]
pub const fn default_user_snippets_dir() -> Option<PathBuf> {
    None
}

#[cfg(test)]
// Snippet bodies like `${var}` look like format args to this lint; they are
// fixture data, not formatting mistakes.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::*;
    use crate::config::schema::ParamKind;

    /// Parse a pack fixture; the test fails on any schema error.
    fn file(origin: &str, text: &str) -> SnippetFile {
        parse_snippet_file(origin, text).expect("fixture pack should parse")
    }

    /// Compile a single-source set; the test fails on any error.
    fn compile_one(origin: &str, text: &str) -> SnippetSet {
        let f = file(origin, text);
        SnippetSet::compile(&[(origin, &f)]).expect("fixture pack should compile")
    }

    /// Compile a single-source set that must fail; returns the error.
    fn compile_err(origin: &str, text: &str) -> ConfigError {
        let f = file(origin, text);
        SnippetSet::compile(&[(origin, &f)]).expect_err("fixture pack should be rejected")
    }

    /// The compiled builtin pack for a language id.
    fn builtin(id: &str) -> SnippetSet {
        let profile = crate::lang::profile::find(id).expect("builtin profile should exist");
        compile_one(&format!("builtin:{id}"), profile.snippet_source)
    }

    #[test]
    fn builtin_rust_pack_compiles() {
        let set = builtin("rust");
        assert!(set.len() >= 30, "rust pack has {} triggers", set.len());
        let fori = set.get("fori").expect("rust pack should define fori");
        assert_eq!(
            fori.params,
            vec![
                ParamDef {
                    name: "var".to_owned(),
                    kind: ParamKind::Ident,
                    default: Some("i".to_owned()),
                },
                ParamDef {
                    name: "end".to_owned(),
                    kind: ParamKind::Int,
                    default: Some("10".to_owned()),
                },
            ]
        );
    }

    #[test]
    fn builtin_python_pack_compiles() {
        let set = builtin("python");
        assert!(set.len() >= 30, "python pack has {} triggers", set.len());
        assert!(!set.is_empty());
    }

    #[test]
    fn later_source_overrides_earlier() {
        let base = file(
            "base",
            "[meta]\nlanguage = \"rust\"\n[snippets.ret]\ndescription = \"old\"\nbody = \"return $1;\"\n",
        );
        let over = file(
            "over",
            "[meta]\nlanguage = \"rust\"\n[snippets.ret]\ndescription = \"new\"\nbody = \"return $0\"\n",
        );
        let set = SnippetSet::compile(&[("base", &base), ("over", &over)])
            .expect("override pack should compile");
        assert_eq!(set.len(), 1);
        let ret = set.get("ret").expect("ret should survive the merge");
        assert_eq!(ret.description.as_deref(), Some("new"));
    }

    #[test]
    fn disabled_removes_earlier_trigger() {
        let base = file(
            "base",
            "[meta]\nlanguage = \"rust\"\n[snippets.ret]\nbody = \"return $1;\"\n",
        );
        let over = file(
            "over",
            "[meta]\nlanguage = \"rust\"\n[snippets.ret]\ndisabled = true\n",
        );
        let set = SnippetSet::compile(&[("base", &base), ("over", &over)])
            .expect("disabling pack should compile");
        assert!(set.get("ret").is_none());
        assert!(set.is_empty());
    }

    #[test]
    fn disabled_without_body_or_prior_definition_is_ok() {
        // Disabling a trigger nothing defined, with no body: still fine.
        let set = compile_one(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.ghost]\ndisabled = true\n",
        );
        assert!(set.is_empty());
    }

    #[test]
    fn missing_body_is_error() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.ret]\ndescription = \"no body\"\n",
        );
        match err {
            ConfigError::Invalid {
                trigger, message, ..
            } => {
                assert_eq!(trigger, "ret");
                assert_eq!(message, "missing body");
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn undeclared_param_reference_is_error() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.pr]\nbody = \"print(${msg})\"\n",
        );
        match err {
            ConfigError::Invalid { message, .. } => {
                assert_eq!(message, "body references undeclared parameter `msg`");
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn declared_but_unreferenced_param_is_error() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.pr]\nparams = [{ name = \"msg\" }]\nbody = \"print()\"\n",
        );
        match err {
            ConfigError::Invalid { message, .. } => {
                assert_eq!(message, "parameter `msg` is never referenced in the body");
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_param_names_are_error() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.pr]\nparams = [{ name = \"msg\" }, { name = \"msg\" }]\nbody = \"print(${msg})\"\n",
        );
        match err {
            ConfigError::Invalid { message, .. } => {
                assert_eq!(message, "duplicate parameter `msg`");
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn param_named_i_is_reserved() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.f]\nparams = [{ name = \"i\" }]\nbody = \"f${i}\"\n",
        );
        match err {
            ConfigError::Invalid { message, .. } => {
                assert!(
                    message.contains("reserved for the repetition index"),
                    "{message}"
                );
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn param_named_value_is_reserved() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.f]\nparams = [{ name = \"value\" }]\nbody = \"f${value}\"\n",
        );
        match err {
            ConfigError::Invalid { message, .. } => {
                assert!(
                    message.contains("reserved for the data-driven repeat item"),
                    "{message}"
                );
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn invalid_trigger_word_is_error() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.\"for-i\"]\nbody = \"x\"\n",
        );
        match err {
            ConfigError::Invalid { trigger, .. } => assert_eq!(trigger, "for-i"),
            other => panic!("expected Invalid, got {other:?}"),
        }
        // Leading digit is equally invalid.
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.\"9lives\"]\nbody = \"x\"\n",
        );
        assert!(matches!(err, ConfigError::Invalid { trigger, .. } if trigger == "9lives"));
    }

    #[test]
    fn bad_template_body_is_snippet_error() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.pr]\nbody = \"print(${)\"\n",
        );
        match err {
            ConfigError::Snippet {
                trigger, origin, ..
            } => {
                assert_eq!(trigger, "pr");
                assert_eq!(origin, "user");
            }
            other => panic!("expected Snippet, got {other:?}"),
        }
    }

    #[test]
    fn variant_may_omit_params_but_not_invent_them() {
        // A variant that omits a declared param is fine.
        let set = compile_one(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.fn]\n",
                "params = [{ name = \"name\", kind = \"ident\" }]\n",
                "body = \"fn ${name}() {}\"\n",
                "[[snippets.fn.variants]]\ncontext = \"type-body\"\nbody = \"fn method(&self) {}\"\n",
            ),
        );
        let compiled = set.get("fn").expect("fn should compile");
        assert_eq!(compiled.variants.len(), 1);
        assert_eq!(compiled.variants[0].context, "type-body");

        // A variant referencing an undeclared param is rejected.
        let err = compile_err(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.fn]\n",
                "body = \"fn f() {}\"\n",
                "[[snippets.fn.variants]]\ncontext = \"type-body\"\nbody = \"fn ${name}(&self) {}\"\n",
            ),
        );
        assert!(matches!(
            err,
            ConfigError::Invalid { message, .. }
                if message == "body references undeclared parameter `name`"
        ));
    }

    #[test]
    fn scope_names_round_trip() {
        for scope in [
            Scope::Root,
            Scope::TypeBody,
            Scope::FnBody,
            Scope::Loop,
            Scope::Conditional,
            Scope::MatchBody,
            Scope::TryBody,
        ] {
            assert_eq!(parse_scope_name(scope_name(scope)), Some(scope));
        }
        assert_eq!(parse_scope_name("unknown"), None);
        assert_eq!(parse_scope_name("TypeBody"), None);
        assert_eq!(scope_name(Scope::Unknown), "unknown");
    }

    #[test]
    fn declared_scope_is_compiled() {
        let set = compile_one(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.impl]\nscope = \"type-body\"\nbody = \"impl X {\\n\\t${child}\\n}\"\n",
        );
        let compiled = set.get("impl").expect("impl should compile");
        assert_eq!(compiled.scope.as_deref(), Some("type-body"));
    }

    #[test]
    fn unknown_scope_is_error() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.impl]\nscope = \"impl-body\"\nbody = \"impl X {}\"\n",
        );
        match err {
            ConfigError::Invalid {
                trigger, message, ..
            } => {
                assert_eq!(trigger, "impl");
                assert!(
                    message.starts_with("unknown scope `impl-body`"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn unknown_variant_context_is_error() {
        let err = compile_err(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.fn]\n",
                "body = \"fn f() {}\"\n",
                "[[snippets.fn.variants]]\ncontext = \"impl\"\nbody = \"fn m(&self) {}\"\n",
            ),
        );
        match err {
            ConfigError::Invalid {
                trigger, message, ..
            } => {
                assert_eq!(trigger, "fn");
                assert!(
                    message.starts_with("unknown variant context `impl`"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn ts_variant_context_is_accepted() {
        let set = compile_one(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.fn]\n",
                "body = \"fn f() {}\"\n",
                "[[snippets.fn.variants]]\ncontext = \"ts:impl_item\"\nbody = \"fn m(&self) {}\"\n",
            ),
        );
        let compiled = set.get("fn").expect("fn should compile");
        assert_eq!(compiled.variants[0].context, "ts:impl_item");
    }

    #[test]
    fn bad_toml_error_carries_origin() {
        let err = parse_snippet_file("user:/tmp/rust.toml", "not toml [")
            .expect_err("garbage should not parse");
        match err {
            ConfigError::Toml { origin, .. } => assert_eq!(origin, "user:/tmp/rust.toml"),
            other => panic!("expected Toml, got {other:?}"),
        }
    }

    #[test]
    fn resolve_word_prefers_longest_trigger() {
        let set = compile_one(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.let]\nbody = \"let $1 = $2;\"\n",
                "[snippets.letm]\nbody = \"let mut $1 = $2;\"\n",
            ),
        );
        let (snippet, rest) = set.resolve_word("letmax").expect("letmax should resolve");
        assert_eq!(snippet.trigger, "letm");
        assert_eq!(rest, "ax");

        let (snippet, rest) = set.resolve_word("letx").expect("letx should resolve");
        assert_eq!(snippet.trigger, "let");
        assert_eq!(rest, "x");

        let (snippet, rest) = set.resolve_word("letm").expect("letm should resolve");
        assert_eq!(snippet.trigger, "letm");
        assert_eq!(rest, "");

        assert!(set.resolve_word("zzz").is_none());
    }
}
