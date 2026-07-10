//! Compiling snippet-pack files into validated, mergeable trigger sets.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::config::schema::{FileDef, OnExists, ParamDef, SnippetDef, SnippetFile};
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
    /// Import requirement specs — raw strings, validated for the pack's
    /// language at compile time when supported (see `lang::imports`); they
    /// are formatted into import lines at the point of use.
    pub requires: Vec<String>,
    /// Set when this trigger is a **composite** (alias): its expansion is an
    /// abbreviation re-expanded at render time rather than [`Self::template`]
    /// (which is empty for a composite). See `COMPOSITE_SNIPPETS.md`.
    pub composite: Option<CompiledComposite>,
    /// Set when this trigger is a **multi-file** snippet: its content spans
    /// several files, resolved by the `scaffold` pipeline rather than expanded
    /// into a single tree. See `MULTIFILE_SNIPPETS.md`.
    pub multifile: Option<CompiledMultiFile>,
}

/// A multi-file snippet: an ordered set of file pieces, each targeting a file.
#[derive(Debug, Clone)]
pub struct CompiledMultiFile {
    /// The file pieces, in declaration order.
    pub files: Vec<CompiledFilePiece>,
}

/// One file piece of a multi-file snippet.
#[derive(Debug, Clone)]
pub struct CompiledFilePiece {
    /// The section label (`[snippets.x.files.<label>]`), for diagnostics.
    pub label: String,
    /// Where this piece's content is written.
    pub target: FileTarget,
    /// The content to render for this file (a body template or a composite).
    pub content: PieceContent,
    /// What to do when the target already exists.
    pub on_exists: OnExists,
    /// The container name to inject into (`${param}`-templated), if any.
    pub into: Option<String>,
    /// Text prepended only when the target file is created.
    pub on_create: Option<String>,
    /// Whether this piece receives the tabstops/final cursor.
    pub primary: bool,
}

/// Where a multi-file piece writes.
#[derive(Debug, Clone)]
pub enum FileTarget {
    /// A path relative to the scaffold root, still `${param}`-templated.
    Path(String),
    /// The invocation's current file.
    SelfFile,
    /// A sibling of the current file with this extension (e.g. `".cpp"`).
    Sibling(String),
}

/// The renderable content of a multi-file piece (mirrors `body` vs `expands`).
#[derive(Debug, Clone)]
pub enum PieceContent {
    /// A template body (LSP snippet syntax + `${param}`).
    Body(String),
    /// A composite-style abbreviation, re-expanded through the engine.
    Expands(String),
}

/// A composite (alias) snippet: an abbreviation that is re-expanded, plus the
/// menu of named option fragments a caller can select into it.
#[derive(Debug, Clone)]
pub struct CompiledComposite {
    /// The base composition abbreviation, with `${name}` parameter markers
    /// substituted at render time before it is re-parsed.
    pub expands: String,
    /// Named option fragments, each an abbreviation merged into the expansion
    /// as a sibling of the base pieces when the caller selects it by name.
    pub options: BTreeMap<String, String>,
}

/// A compiled context-conditional variant.
#[derive(Debug, Clone)]
pub struct CompiledVariant {
    /// Normalized scope name or `ts:`-prefixed raw node kind.
    pub context: String,
    /// The variant's template.
    pub template: Template,
    /// Import requirement specs. `Some` (even empty) **replaces** the
    /// snippet-level `requires` when this variant is selected; `None`
    /// inherits it.
    pub requires: Option<Vec<String>>,
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
                let compiled = compile_snippet(origin, &file.meta.language, trigger, def)?;
                snippets.insert(trigger.clone(), compiled);
            }
        }
        let set = Self { snippets };
        // Composite and multi-file validation are whole-set passes: a fragment
        // may reference any trigger in the *merged* set (including ones a later
        // pack added), so they can only run once the merge is complete.
        set.validate_composites()?;
        set.validate_multifile()?;
        Ok(set)
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

    /// Validate every composite in the merged set: its `expands` and each
    /// option must parse, every trigger they reference must resolve, and the
    /// composite reference graph must be acyclic.
    ///
    /// This is the `codemmet check` guard for composites and the invariant the
    /// renderer relies on — because definitional cycles are rejected here, the
    /// render-time expansion of a composite always terminates (the runtime
    /// depth cap is only a defence-in-depth backstop).
    fn validate_composites(&self) -> Result<(), ConfigError> {
        // trigger -> composites it references (edges for the cycle graph).
        let mut edges: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for snippet in self.snippets.values() {
            let Some(composite) = &snippet.composite else {
                continue;
            };
            let invalid = |message: String| ConfigError::Invalid {
                trigger: snippet.trigger.clone(),
                origin: "composite".to_owned(),
                message,
            };
            let fragments = std::iter::once(("expands", composite.expands.as_str())).chain(
                composite
                    .options
                    .iter()
                    .map(|(name, frag)| (name.as_str(), frag.as_str())),
            );
            let mut refs: BTreeSet<&str> = BTreeSet::new();
            for (label, fragment) in fragments {
                // Substitute a safe identifier for every `${name}` so the
                // fragment parses; the concrete values do not affect which
                // *triggers* it references. `${child}` becomes its parse-safe
                // sentinel, which `collect_referenced_triggers` skips.
                let text = substitute_markers(&substitute_child_slot(fragment), |_| "x".to_owned());
                let ast = crate::abbr::parse(&text).map_err(|err| {
                    invalid(format!(
                        "{label} fragment `{fragment}` does not parse: {err}"
                    ))
                })?;
                collect_referenced_triggers(self, &ast.0, &mut refs, &invalid)?;
            }
            edges.insert(snippet.trigger.as_str(), refs);
        }
        detect_cycles(&edges)
    }

    /// Validate that every `expands` piece of a multi-file snippet references
    /// only triggers that resolve in the merged set. Like composites, this must
    /// run whole-set (a piece may reference a trigger a later pack provides).
    /// No cycle check is needed: a multi-file snippet is never referenced by an
    /// abbreviation, so it cannot participate in one.
    fn validate_multifile(&self) -> Result<(), ConfigError> {
        for snippet in self.snippets.values() {
            let Some(multifile) = &snippet.multifile else {
                continue;
            };
            let invalid = |message: String| ConfigError::Invalid {
                trigger: snippet.trigger.clone(),
                origin: "multifile".to_owned(),
                message,
            };
            for piece in &multifile.files {
                let PieceContent::Expands(expands) = &piece.content else {
                    continue;
                };
                // Substitute a safe identifier for every `${name}` so the
                // fragment parses; the concrete values sit in argument slots and
                // do not affect which *triggers* it references.
                let text = substitute_markers(&substitute_child_slot(expands), |_| "x".to_owned());
                let ast = crate::abbr::parse(&text).map_err(|err| {
                    invalid(format!(
                        "file `{}` expands `{expands}` does not parse: {err}",
                        piece.label
                    ))
                })?;
                let mut refs: BTreeSet<&str> = BTreeSet::new();
                collect_referenced_triggers(self, &ast.0, &mut refs, &invalid)?;
            }
        }
        Ok(())
    }
}

/// Resolve every trigger a composite fragment's nodes reference, recording the
/// composite-to-composite edges and erroring on an unknown trigger.
fn collect_referenced_triggers<'s>(
    set: &'s SnippetSet,
    nodes: &[crate::abbr::AbbrNode],
    refs: &mut BTreeSet<&'s str>,
    invalid: &impl Fn(String) -> ConfigError,
) -> Result<(), ConfigError> {
    for node in nodes {
        // The `${child}` insertion point parses as this sentinel word; it is
        // not a trigger, so skip it (the renderer replaces it with the extras).
        if node.word == CHILD_SENTINEL {
            collect_referenced_triggers(set, &node.children, refs, invalid)?;
            continue;
        }
        match set.resolve_word(&node.word) {
            None => {
                return Err(invalid(format!(
                    "references unknown trigger `{}`",
                    node.word
                )));
            }
            Some((resolved, _)) if resolved.composite.is_some() => {
                refs.insert(resolved.trigger.as_str());
            }
            Some(_) => {}
        }
        collect_referenced_triggers(set, &node.children, refs, invalid)?;
    }
    Ok(())
}

/// Report the first cycle in the composite reference graph, if any.
fn detect_cycles(edges: &BTreeMap<&str, BTreeSet<&str>>) -> Result<(), ConfigError> {
    let mut visited: BTreeSet<&str> = BTreeSet::new();
    let mut on_stack: BTreeSet<&str> = BTreeSet::new();
    for &start in edges.keys() {
        visit_for_cycle(start, edges, &mut visited, &mut on_stack)?;
    }
    Ok(())
}

/// DFS one node; `on_stack` holds the current path so a back-edge is a cycle.
fn visit_for_cycle<'s>(
    node: &'s str,
    edges: &BTreeMap<&'s str, BTreeSet<&'s str>>,
    visited: &mut BTreeSet<&'s str>,
    on_stack: &mut BTreeSet<&'s str>,
) -> Result<(), ConfigError> {
    if visited.contains(node) {
        return Ok(());
    }
    on_stack.insert(node);
    for &next in edges.get(node).into_iter().flatten() {
        if on_stack.contains(next) {
            return Err(ConfigError::Invalid {
                trigger: node.to_owned(),
                origin: "composite".to_owned(),
                message: format!("composite cycle: `{node}` expands through `{next}`"),
            });
        }
        visit_for_cycle(next, edges, visited, on_stack)?;
    }
    on_stack.remove(node);
    visited.insert(node);
    Ok(())
}

/// Replace every `${name}` marker in `text` using `value`.
///
/// Shared by validation (a safe dummy) and the renderer (the bound argument or
/// default). A `${` with no closing `}` is emitted verbatim.
pub fn substitute_markers(text: &str, mut value: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            out.push_str(&value(&after[..end]));
            rest = &after[end + 1..];
        } else {
            out.push_str(&rest[start..]);
            return out;
        }
    }
    out.push_str(rest);
    out
}

/// The reserved `${child}` insertion-point name in a composite's `expands`.
pub const COMPOSITE_CHILD_NAME: &str = "child";

/// The parse-safe word a composite's `${child}` marker becomes before parsing.
///
/// It is a real WORD (so it parses as a node) that is distinctive enough never
/// to collide with a real trigger; the renderer replaces the resulting node
/// with the composite's extras, and validation skips it.
pub const CHILD_SENTINEL: &str = "CMMTCHILDSLOTCMMT";

/// Rewrite a composite fragment's `${child}` markers to [`CHILD_SENTINEL`] so
/// the fragment parses and the marker survives as a locatable node.
#[must_use]
pub fn substitute_child_slot(text: &str) -> String {
    text.replace("${child}", CHILD_SENTINEL)
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
            Span::Param { name, .. } => Some(name.as_str()),
            _ => None,
        })
}

/// Validate one `requires` list for the pack's language: no duplicates, and
/// (when the language has an import formatter) every spec parses. Packs for
/// languages without a formatter keep their specs verbatim; `codemmet check`
/// warns about those.
fn validate_requires(
    invalid: &impl Fn(String) -> ConfigError,
    language: &str,
    specs: &[String],
) -> Result<(), ConfigError> {
    let mut seen = BTreeSet::new();
    for spec in specs {
        if !seen.insert(spec.as_str()) {
            return Err(invalid(format!("duplicate requires spec `{spec}`")));
        }
        if crate::lang::imports::supports(language) {
            crate::lang::imports::parse_spec(language, spec)
                .map_err(|message| invalid(format!("invalid requires spec `{spec}`: {message}")))?;
        }
    }
    Ok(())
}

/// Validate and compile one non-disabled snippet definition.
///
/// `language` is the pack's `meta.language`, used to validate `requires`
/// specs against that language's import-spec grammar.
fn compile_snippet(
    origin: &str,
    language: &str,
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

    validate_requires(&invalid, language, &def.requires)?;

    // A multi-file snippet carries `[files.*]` sections instead of a body.
    if !def.files.is_empty() {
        return compile_multifile(origin, trigger, def);
    }

    // A composite carries `expands` (an abbreviation) instead of a `body`.
    if def.expands.is_some() {
        return compile_composite(origin, trigger, def);
    }

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
        if let Some(requires) = &variant.requires {
            validate_requires(&invalid, language, requires)?;
        }
        variants.push(CompiledVariant {
            context: variant.context.clone(),
            template: parse_template(&variant.body).map_err(snippet_err)?,
            requires: variant.requires.clone(),
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
        if param.name == "outer" {
            return Err(invalid(
                "parameter name `outer` is reserved for the enclosing repeat item `${outer}`"
                    .to_owned(),
            ));
        }
        if !declared.insert(param.name.as_str()) {
            return Err(invalid(format!("duplicate parameter `{}`", param.name)));
        }
    }

    check_param_references(&invalid, def, &template, &variants, &declared)?;

    Ok(CompiledSnippet {
        trigger: trigger.to_owned(),
        description: def.description.clone(),
        params: def.params.clone(),
        template,
        scope: def.scope.clone(),
        variants,
        requires: def.requires.clone(),
        composite: None,
        multifile: None,
    })
}

/// Cross-check `${name}` references against declared params: every reference
/// (default body and variants) must be declared, and every declared param
/// must appear in the default body (variants may omit params).
fn check_param_references(
    invalid: &impl Fn(String) -> ConfigError,
    def: &SnippetDef,
    template: &Template,
    variants: &[CompiledVariant],
    declared: &BTreeSet<&str>,
) -> Result<(), ConfigError> {
    for referenced in std::iter::once(template)
        .chain(variants.iter().map(|v| &v.template))
        .flat_map(referenced_params)
    {
        if !declared.contains(referenced) {
            return Err(invalid(format!(
                "body references undeclared parameter `{referenced}`"
            )));
        }
    }
    let default_refs: BTreeSet<&str> = referenced_params(template).collect();
    for param in &def.params {
        if !default_refs.contains(param.name.as_str()) {
            return Err(invalid(format!(
                "parameter `{}` is never referenced in the body",
                param.name
            )));
        }
    }
    Ok(())
}

/// Validate and compile a composite (`expands`) snippet definition.
///
/// A composite has no template body, scope, or variants — its expansion is an
/// abbreviation re-expanded at render time. This checks the field exclusions
/// and parameter rules; the *reference* checks (every trigger resolves, no
/// definitional cycle) run afterwards over the merged set in
/// [`SnippetSet::validate_composites`], because a composite may reference a
/// trigger that a later pack in the merge provides.
fn compile_composite(
    origin: &str,
    trigger: &str,
    def: &SnippetDef,
) -> Result<CompiledSnippet, ConfigError> {
    let invalid = |message: String| ConfigError::Invalid {
        trigger: trigger.to_owned(),
        origin: origin.to_owned(),
        message,
    };
    let expands = def
        .expands
        .as_deref()
        .expect("caller checked expands.is_some()");

    if def.body.is_some() {
        return Err(invalid(
            "a composite (`expands`) cannot also have a `body`".to_owned(),
        ));
    }
    if def.scope.is_some() {
        return Err(invalid(
            "a composite (`expands`) cannot declare a `scope`; the construct it \
             expands to owns the child scope"
                .to_owned(),
        ));
    }
    if !def.variants.is_empty() {
        return Err(invalid(
            "a composite (`expands`) cannot declare `variants`".to_owned(),
        ));
    }

    // Parameter rules mirror the template path: reserved names, no duplicates,
    // every `${name}` declared, every declared param referenced.
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
        if param.name == "outer" {
            return Err(invalid(
                "parameter name `outer` is reserved for the enclosing repeat item `${outer}`"
                    .to_owned(),
            ));
        }
        if param.name == COMPOSITE_CHILD_NAME {
            return Err(invalid(
                "parameter name `child` is reserved for the `${child}` insertion point".to_owned(),
            ));
        }
        if !declared.insert(param.name.as_str()) {
            return Err(invalid(format!("duplicate parameter `{}`", param.name)));
        }
    }

    // `${child}` is the explicit insertion point for options and freeform
    // children; it belongs in `expands` and at most once.
    if expands.matches("${child}").count() > 1 {
        return Err(invalid(
            "a composite may use `${child}` at most once in `expands`".to_owned(),
        ));
    }
    if def.options.values().any(|f| f.contains("${child}")) {
        return Err(invalid(
            "`${child}` may only appear in `expands`, not in an option fragment".to_owned(),
        ));
    }

    let fragments = std::iter::once(expands).chain(def.options.values().map(String::as_str));
    let mut referenced: BTreeSet<String> = BTreeSet::new();
    for fragment in fragments {
        // `${child}` is not a parameter reference; hide it before scanning.
        for name in param_markers(&substitute_child_slot(fragment)) {
            if !declared.contains(name.as_str()) {
                return Err(invalid(format!(
                    "composite references undeclared parameter `{name}`"
                )));
            }
            referenced.insert(name);
        }
    }
    for param in &def.params {
        if !referenced.contains(&param.name) {
            return Err(invalid(format!(
                "parameter `{}` is never referenced by the composite",
                param.name
            )));
        }
    }

    Ok(CompiledSnippet {
        trigger: trigger.to_owned(),
        description: def.description.clone(),
        params: def.params.clone(),
        template: Template { lines: Vec::new() },
        scope: None,
        variants: Vec::new(),
        // A composite's own requires; the requires of the snippets its
        // expansion references are aggregated at render time.
        requires: def.requires.clone(),
        composite: Some(CompiledComposite {
            expands: expands.to_owned(),
            options: def.options.clone(),
        }),
        multifile: None,
    })
}

/// Validate and compile a multi-file (`[files.*]`) snippet definition.
///
/// A multi-file snippet has no top-level `body`/`expands`/`composite`; each
/// `[files.<label>]` section targets a file (a `path` template or a `self`/
/// sibling `target`) and carries either a `body` or an `expands`. Parameters
/// are shared across the sections; every declared param must be referenced by
/// at least one section (its path, body, or expands), and every `${name}` a
/// section references must be declared.
fn compile_multifile(
    origin: &str,
    trigger: &str,
    def: &SnippetDef,
) -> Result<CompiledSnippet, ConfigError> {
    let invalid = |message: String| ConfigError::Invalid {
        trigger: trigger.to_owned(),
        origin: origin.to_owned(),
        message,
    };

    if def.body.is_some() || def.expands.is_some() {
        return Err(invalid(
            "a multi-file snippet (`[files]`) cannot also have a top-level \
             `body` or `expands`"
                .to_owned(),
        ));
    }
    if def.scope.is_some() || !def.variants.is_empty() || !def.options.is_empty() {
        return Err(invalid(
            "a multi-file snippet cannot declare `scope`, `variants`, or `options`".to_owned(),
        ));
    }

    // Parameter rules mirror the other paths: reserved names, no duplicates.
    let mut declared = BTreeSet::new();
    for param in &def.params {
        if matches!(param.name.as_str(), "i" | "value" | COMPOSITE_CHILD_NAME) {
            return Err(invalid(format!(
                "parameter name `{}` is reserved",
                param.name
            )));
        }
        if !declared.insert(param.name.as_str()) {
            return Err(invalid(format!("duplicate parameter `{}`", param.name)));
        }
    }

    let mut files = Vec::with_capacity(def.files.len());
    let mut referenced: BTreeSet<String> = BTreeSet::new();
    let mut primaries = 0usize;
    // `def.files` is a BTreeMap, so sections compile in label order.
    for (label, file) in &def.files {
        let piece = compile_file_piece(&invalid, label, file, &declared, &mut referenced)?;
        if piece.primary {
            primaries += 1;
        }
        files.push(piece);
    }

    if files.is_empty() {
        return Err(invalid(
            "a multi-file snippet needs at least one `[files.<label>]` section".to_owned(),
        ));
    }
    if primaries > 1 {
        return Err(invalid(
            "at most one file section may set `primary = true`".to_owned(),
        ));
    }
    for param in &def.params {
        if !referenced.contains(&param.name) {
            return Err(invalid(format!(
                "parameter `{}` is never referenced by any file section",
                param.name
            )));
        }
    }

    Ok(CompiledSnippet {
        trigger: trigger.to_owned(),
        description: def.description.clone(),
        params: def.params.clone(),
        template: Template { lines: Vec::new() },
        scope: None,
        variants: Vec::new(),
        composite: None,
        requires: def.requires.clone(),
        multifile: Some(CompiledMultiFile { files }),
    })
}

/// Compile and validate one `[files.<label>]` section, recording the params it
/// references into `referenced`.
fn compile_file_piece(
    invalid: &impl Fn(String) -> ConfigError,
    label: &str,
    file: &FileDef,
    declared: &BTreeSet<&str>,
    referenced: &mut BTreeSet<String>,
) -> Result<CompiledFilePiece, ConfigError> {
    // Exactly one of `path` / `target`, and one of `body` / `expands`.
    let target = match (&file.path, &file.target) {
        (Some(_), Some(_)) => {
            return Err(invalid(format!(
                "file section `{label}` sets both `path` and `target`; use one"
            )));
        }
        (Some(path), None) => FileTarget::Path(path.clone()),
        (None, Some(target)) => parse_target(invalid, label, target)?,
        (None, None) => {
            return Err(invalid(format!(
                "file section `{label}` needs a `path` or a `target`"
            )));
        }
    };
    // The params this section's content references. A `body` is a *template*
    // (so its `${1:x}` tabstops must not be read as params — use the parsed
    // param spans); an `expands` is an abbreviation (scan its `${name}`s).
    let (content, content_refs): (PieceContent, Vec<String>) = match (&file.body, &file.expands) {
        (Some(_), Some(_)) => {
            return Err(invalid(format!(
                "file section `{label}` sets both `body` and `expands`; use one"
            )));
        }
        (Some(body), None) => {
            // Parse now so a bad template fails `check`, not at scaffold time.
            let template = parse_template(body).map_err(|source| ConfigError::Snippet {
                trigger: label.to_owned(),
                origin: String::new(),
                source,
            })?;
            let refs = referenced_params(&template)
                .map(ToOwned::to_owned)
                .collect();
            (PieceContent::Body(body.clone()), refs)
        }
        (None, Some(expands)) => {
            crate::abbr::parse(expands).map_err(|err| {
                invalid(format!(
                    "file section `{label}` `expands` does not parse: {err}"
                ))
            })?;
            (
                PieceContent::Expands(expands.clone()),
                param_markers(expands),
            )
        }
        (None, None) => {
            return Err(invalid(format!(
                "file section `{label}` needs a `body` or an `expands`"
            )));
        }
    };

    // Collect and check every `${name}` this section references (path + `into`
    // + content).
    let path_refs = file.path.as_deref().map(param_markers).unwrap_or_default();
    let into_refs = file.into.as_deref().map(param_markers).unwrap_or_default();
    for name in path_refs.into_iter().chain(into_refs).chain(content_refs) {
        if !declared.contains(name.as_str()) {
            return Err(invalid(format!(
                "file section `{label}` references undeclared parameter `{name}`"
            )));
        }
        referenced.insert(name);
    }

    Ok(CompiledFilePiece {
        label: label.to_owned(),
        target,
        content,
        on_exists: file.on_exists,
        into: file.into.clone(),
        on_create: file.on_create.clone(),
        primary: file.primary,
    })
}

/// Parse a `target` value: `"self"`, or `.<ext>` for a sibling by extension.
fn parse_target(
    invalid: &impl Fn(String) -> ConfigError,
    label: &str,
    target: &str,
) -> Result<FileTarget, ConfigError> {
    if target == "self" {
        Ok(FileTarget::SelfFile)
    } else if let Some(ext) = target.strip_prefix('.').filter(|e| !e.is_empty()) {
        Ok(FileTarget::Sibling(format!(".{ext}")))
    } else {
        Err(invalid(format!(
            "file section `{label}` has invalid `target` {target:?} \
             (expected `self` or a sibling extension like `.cpp`)"
        )))
    }
}

/// The `${name}` parameter markers a composite fragment references, in order.
///
/// Only the `${name}` form is recognized in a composite abbreviation; a `$1`
/// or bare `$` is left for the underlying snippets to interpret after
/// re-parsing, so this scans for `${...}` and yields the interior name.
fn param_markers(fragment: &str) -> Vec<String> {
    let mut names = Vec::new();
    let _ = substitute_markers(fragment, |name| {
        names.push(name.to_owned());
        String::new()
    });
    names
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
    fn param_named_outer_is_reserved() {
        let err = compile_err(
            "user",
            "[meta]\nlanguage = \"rust\"\n[snippets.f]\nparams = [{ name = \"outer\" }]\nbody = \"f${outer}\"\n",
        );
        match err {
            ConfigError::Invalid { message, .. } => {
                assert!(
                    message.contains("reserved for the enclosing repeat item"),
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

    // --- requires -------------------------------------------------------------

    #[test]
    fn requires_compiles_and_is_recorded() {
        let set = compile_one(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.hmap]\n",
                "requires = [\"std::collections::HashMap\"]\n",
                "body = \"let m = HashMap::new();$0\"\n",
            ),
        );
        let hmap = set.get("hmap").expect("hmap should compile");
        assert_eq!(hmap.requires, ["std::collections::HashMap"]);
    }

    #[test]
    fn invalid_requires_spec_is_error() {
        let err = compile_err(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.bad]\n",
                "requires = [\"use std::fmt;\"]\n",
                "body = \"x$0\"\n",
            ),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("invalid requires spec")),
            "{err:?}"
        );
    }

    #[test]
    fn duplicate_requires_spec_is_error() {
        let err = compile_err(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.bad]\n",
                "requires = [\"std::fmt::Display\", \"std::fmt::Display\"]\n",
                "body = \"x$0\"\n",
            ),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("duplicate requires")),
            "{err:?}"
        );
    }

    #[test]
    fn variant_requires_compile_and_validate() {
        let set = compile_one(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.disp]\n",
                "requires = [\"std::fmt::Display\"]\n",
                "body = \"x$0\"\n",
                "[[snippets.disp.variants]]\ncontext = \"type-body\"\nrequires = []\nbody = \"y$0\"\n",
            ),
        );
        let disp = set.get("disp").expect("disp should compile");
        assert_eq!(disp.requires, ["std::fmt::Display"]);
        // Declared-empty is preserved as `Some([])` (replace), not `None`.
        assert_eq!(disp.variants[0].requires, Some(Vec::new()));

        let err = compile_err(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.disp]\nbody = \"x$0\"\n",
                "[[snippets.disp.variants]]\ncontext = \"type-body\"\nrequires = [\"nope;\"]\nbody = \"y$0\"\n",
            ),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("invalid requires spec")),
            "{err:?}"
        );
    }

    #[test]
    fn variant_without_requires_stays_inheriting_none() {
        let set = compile_one(
            "user",
            concat!(
                "[meta]\nlanguage = \"rust\"\n",
                "[snippets.fn]\nbody = \"fn f() {}\"\n",
                "[[snippets.fn.variants]]\ncontext = \"type-body\"\nbody = \"fn m(&self) {}\"\n",
            ),
        );
        assert_eq!(
            set.get("fn").expect("fn compiles").variants[0].requires,
            None
        );
    }

    #[test]
    fn composite_may_declare_requires() {
        let set = compile_one(
            "user",
            &composite_pack(concat!(
                "[snippets.bar]\n",
                "requires = [\"std::fmt::Write\"]\n",
                "expands = \"foo\"\n",
            )),
        );
        assert_eq!(
            set.get("bar").expect("bar compiles").requires,
            ["std::fmt::Write"]
        );
    }

    #[test]
    fn unsupported_language_requires_are_kept_verbatim() {
        // JavaScript has no import formatter yet: specs pass through
        // unvalidated (`codemmet check` warns about them).
        let set = compile_one(
            "user",
            concat!(
                "[meta]\nlanguage = \"javascript\"\n",
                "[snippets.rf]\n",
                "requires = [\"node:fs\"]\n",
                "body = \"readFileSync($0)\"\n",
            ),
        );
        assert_eq!(set.get("rf").expect("rf compiles").requires, ["node:fs"]);
    }

    // --- composites ----------------------------------------------------------

    /// A pack fixture with a base trigger `foo` plus whatever `extra` adds.
    fn composite_pack(extra: &str) -> String {
        format!("[meta]\nlanguage = \"rust\"\n[snippets.foo]\nbody = \"foo();$0\"\n{extra}")
    }

    #[test]
    fn composite_compiles_and_records_expands_and_options() {
        let set = compile_one(
            "user",
            &composite_pack(concat!(
                "[snippets.bar]\n",
                "description = \"two foos\"\n",
                "expands = \"foo+foo\"\n",
                "[snippets.bar.options]\n",
                "more = \"foo\"\n",
            )),
        );
        let bar = set.get("bar").expect("bar should compile");
        let composite = bar.composite.as_ref().expect("bar is a composite");
        assert_eq!(composite.expands, "foo+foo");
        assert_eq!(
            composite.options.get("more").map(String::as_str),
            Some("foo")
        );
        assert!(bar.template.lines.is_empty(), "a composite has no template");
    }

    #[test]
    fn composite_with_body_is_error() {
        let err = compile_err(
            "user",
            &composite_pack("[snippets.bar]\nexpands = \"foo\"\nbody = \"x\"\n"),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("cannot also have a `body`")),
            "{err:?}"
        );
    }

    #[test]
    fn composite_with_scope_is_error() {
        let err = compile_err(
            "user",
            &composite_pack("[snippets.bar]\nexpands = \"foo\"\nscope = \"fn-body\"\n"),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("cannot declare a `scope`")),
            "{err:?}"
        );
    }

    #[test]
    fn composite_undeclared_param_is_error() {
        let err = compile_err(
            "user",
            &composite_pack("[snippets.bar]\nexpands = \"foo:${name}\"\n"),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message == "composite references undeclared parameter `name`"),
            "{err:?}"
        );
    }

    #[test]
    fn composite_unreferenced_param_is_error() {
        let err = compile_err(
            "user",
            &composite_pack(concat!(
                "[snippets.bar]\n",
                "params = [{ name = \"name\", kind = \"ident\" }]\n",
                "expands = \"foo\"\n",
            )),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message == "parameter `name` is never referenced by the composite"),
            "{err:?}"
        );
    }

    #[test]
    fn composite_referencing_unknown_trigger_is_error() {
        let err = compile_err(
            "user",
            &composite_pack("[snippets.bar]\nexpands = \"foo+nope\"\n"),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message == "references unknown trigger `nope`"),
            "{err:?}"
        );
    }

    #[test]
    fn composite_cycle_is_error() {
        let err = compile_err(
            "user",
            &composite_pack(concat!(
                "[snippets.a]\nexpands = \"b\"\n",
                "[snippets.b]\nexpands = \"a\"\n",
            )),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("composite cycle")),
            "{err:?}"
        );
    }

    #[test]
    fn composite_referencing_a_composite_is_allowed_when_acyclic() {
        let set = compile_one(
            "user",
            &composite_pack(concat!(
                "[snippets.a]\nexpands = \"foo\"\n",
                "[snippets.b]\nexpands = \"a+foo\"\n",
            )),
        );
        assert!(set.get("a").and_then(|s| s.composite.as_ref()).is_some());
        assert!(set.get("b").and_then(|s| s.composite.as_ref()).is_some());
    }

    #[test]
    fn composite_with_unparseable_expansion_is_error() {
        let err = compile_err(
            "user",
            &composite_pack("[snippets.bar]\nexpands = \"foo>\"\n"),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("does not parse")),
            "{err:?}"
        );
    }

    #[test]
    fn composite_with_explicit_child_slot_compiles() {
        // `${child}` is a valid insertion point, not an undeclared parameter.
        let set = compile_one(
            "user",
            &composite_pack("[snippets.bar]\nexpands = \"foo>${child}\"\n"),
        );
        let bar = set.get("bar").expect("bar should compile");
        assert_eq!(
            bar.composite.as_ref().map(|c| c.expands.as_str()),
            Some("foo>${child}")
        );
    }

    #[test]
    fn composite_reserved_child_param_is_error() {
        let err = compile_err(
            "user",
            &composite_pack(
                "[snippets.bar]\nparams = [{ name = \"child\", kind = \"ident\" }]\nexpands = \"foo\"\n",
            ),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("`child` is reserved")),
            "{err:?}"
        );
    }

    #[test]
    fn composite_child_slot_in_option_is_error() {
        let err = compile_err(
            "user",
            &composite_pack(
                "[snippets.bar]\nexpands = \"foo\"\n[snippets.bar.options]\nx = \"foo>${child}\"\n",
            ),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("only appear in `expands`")),
            "{err:?}"
        );
    }

    #[test]
    fn composite_multiple_child_slots_is_error() {
        let err = compile_err(
            "user",
            &composite_pack("[snippets.bar]\nexpands = \"foo>${child}+foo>${child}\"\n"),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref message, .. } if message.contains("at most once")),
            "{err:?}"
        );
    }

    // --- multi-file ----------------------------------------------------------

    #[test]
    fn multifile_expands_referencing_unknown_trigger_is_error() {
        let err = compile_err(
            "user",
            &composite_pack(concat!(
                "[snippets.mf.files.a]\n",
                "path = \"a.rs\"\n",
                "expands = \"foo+nope\"\n",
            )),
        );
        assert!(
            matches!(err, ConfigError::Invalid { ref origin, ref message, .. }
                if origin == "multifile" && message == "references unknown trigger `nope`"),
            "{err:?}"
        );
    }

    #[test]
    fn multifile_expands_referencing_a_known_trigger_compiles() {
        // `foo` is defined by `composite_pack`; a later pack could add it too.
        let set = compile_one(
            "user",
            &composite_pack("[snippets.mf.files.a]\npath = \"a.rs\"\nexpands = \"foo+foo\"\n"),
        );
        assert!(set.get("mf").and_then(|s| s.multifile.as_ref()).is_some());
    }

    #[test]
    fn multifile_body_piece_skips_trigger_validation() {
        // A `body` piece is a template, not an abbreviation — its words are not
        // triggers, so `whatever` must not be validated as one.
        let set = compile_one(
            "user",
            &composite_pack("[snippets.mf.files.a]\npath = \"a.rs\"\nbody = \"whatever\"\n"),
        );
        assert!(set.get("mf").and_then(|s| s.multifile.as_ref()).is_some());
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
