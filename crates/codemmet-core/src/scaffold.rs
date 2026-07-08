//! Multi-file snippet expansion: one trigger → coordinated content across
//! several files.
//!
//! A multi-file snippet (`[snippets.<t>.files.<label>]`, see the loader) is
//! *not* expanded into one tree like a composite. Instead each file **piece**
//! is rendered on its own — reusing the ordinary render pipeline — and paired
//! with a resolved target path and an [`OnExists`] intent. The result is a
//! **manifest** ([`FileOutput`] list): a description of what to write where.
//! The caller (CLI `--write`, or the editor) decides create-vs-append/inject
//! by looking at the filesystem; this module never touches disk, so it stays
//! pure and testable.
//!
//! Parameters are resolved to **concrete text** before rendering (a bound
//! argument, else the declared default), because a param typically appears in
//! file *paths* and so must be a real value, and because tabstops cannot be
//! kept in sync across files. Tabstops therefore come only from a piece's own
//! body/`expands`, and only the single **primary** piece keeps them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::abbr::{AbbrNode, Abbreviation, ParseError, Repeat};
use crate::config::loader::{
    CompiledFilePiece, CompiledSnippet, FileTarget, PieceContent, SnippetSet, substitute_markers,
};
use crate::config::schema::{OnExists, ParamKind};
use crate::context::ExpansionContext;
use crate::engine::{Engine, ExpandError};
use crate::snippet::ir::Template;
use crate::snippet::render::{RenderError, TabstopMode, render};
use crate::snippet::template::{TemplateError, parse_template};

/// Inputs a multi-file expansion needs from its environment.
#[derive(Debug, Clone, Copy)]
pub struct ScaffoldOptions<'a> {
    /// Base directory that `path` targets are resolved against.
    pub root: &'a Path,
    /// The file the invocation happened in, for `self`/sibling targets.
    pub current_file: Option<&'a Path>,
}

/// One entry of the expansion manifest: content destined for one file.
#[derive(Debug, Clone)]
pub struct FileOutput {
    /// The section label, for diagnostics and the manifest.
    pub label: String,
    /// The resolved target path (root-joined, or a sibling of the current file).
    pub path: PathBuf,
    /// The rendered content to write or append.
    pub content: String,
    /// Text to prepend **only when the file is created** (already
    /// param-substituted); `None` when the section declared no `on_create`.
    pub on_create: Option<String>,
    /// What to do when the target already exists.
    pub on_exists: OnExists,
    /// For `inject`: the container name to target (already param-substituted).
    pub into: Option<String>,
    /// Whether this is the primary file (the one that keeps tabstops).
    pub primary: bool,
}

/// Why a multi-file expansion failed.
#[derive(Debug, thiserror::Error)]
pub enum ScaffoldError {
    /// The requested language is not registered.
    #[error("unknown language `{0}`")]
    UnknownLanguage(String),
    /// The invocation text did not parse as an abbreviation.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// The invocation was a chain, not a single standalone trigger.
    #[error("a multi-file snippet is invoked standalone, not in a `>`/`+`/`*` chain")]
    NotStandalone,
    /// The trigger does not resolve in the language.
    #[error("unknown trigger `{0}`")]
    UnknownTrigger(String),
    /// The trigger resolves, but is not a multi-file snippet.
    #[error("`{0}` is not a multi-file snippet")]
    NotMultiFile(String),
    /// A `self`/sibling target was used with no current file available.
    #[error(
        "file section `{label}` targets the current file, but none was given \
         (pass `--from <FILE>`)"
    )]
    NoCurrentFile {
        /// The offending section label.
        label: String,
    },
    /// A body piece's template failed to render.
    #[error("file section `{label}`: {source}")]
    Body {
        /// The offending section label.
        label: String,
        /// The underlying template error.
        source: TemplateError,
    },
    /// A body piece failed to render.
    #[error("file section `{label}`: {source}")]
    Render {
        /// The offending section label.
        label: String,
        /// The underlying render error.
        source: RenderError,
    },
    /// An `expands` piece failed to expand.
    #[error("file section `{label}`: cannot expand `{abbr}`: {source}")]
    Expand {
        /// The offending section label.
        label: String,
        /// The (param-substituted) abbreviation that failed.
        abbr: String,
        /// The underlying expansion error.
        source: Box<ExpandError>,
    },
}

/// Expand a multi-file snippet invocation into a file manifest.
///
/// `invocation` is a single standalone trigger with its arguments
/// (`djfeature:blog`); chains (`>`/`+`/`*`) are rejected. `mode` is applied to
/// the primary file only; every other file is rendered in
/// [`TabstopMode::Strip`].
pub fn scaffold(
    engine: &Engine,
    language: &str,
    invocation: &str,
    ctx: &ExpansionContext,
    opts: &ScaffoldOptions,
    mode: TabstopMode,
) -> Result<Vec<FileOutput>, ScaffoldError> {
    let lang = engine
        .registry()
        .language(language)
        .ok_or_else(|| ScaffoldError::UnknownLanguage(language.to_owned()))?;

    let ast = crate::abbr::parse(invocation)?;
    let node = standalone_node(&ast)?;
    let (snippet, remainder) = lang
        .set
        .resolve_word(&node.word)
        .ok_or_else(|| ScaffoldError::UnknownTrigger(node.word.clone()))?;
    let multifile = snippet
        .multifile
        .as_ref()
        .ok_or_else(|| ScaffoldError::NotMultiFile(snippet.trigger.clone()))?;

    let values = bind_values(snippet, &node.args, remainder);
    let primary = choose_primary(&multifile.files);

    let mut out = Vec::with_capacity(multifile.files.len());
    for (i, piece) in multifile.files.iter().enumerate() {
        let is_primary = i == primary;
        let piece_mode = if is_primary { mode } else { TabstopMode::Strip };
        out.push(FileOutput {
            label: piece.label.clone(),
            path: resolve_path(piece, &values, opts)?,
            content: render_piece(engine, language, piece, &values, ctx, piece_mode)?,
            on_create: piece.on_create.as_deref().map(|p| subst(p, &values)),
            on_exists: piece.on_exists,
            into: piece.into.as_deref().map(|p| subst(p, &values)),
            primary: is_primary,
        });
    }
    Ok(out)
}

/// The single standalone node of `ast`, or an error for an empty invocation or
/// a chain (children/siblings/repeat).
fn standalone_node(ast: &Abbreviation) -> Result<&AbbrNode, ScaffoldError> {
    match ast.0.as_slice() {
        [node] if node.children.is_empty() && matches!(node.repeat, Repeat::Count(1)) => Ok(node),
        _ => Err(ScaffoldError::NotStandalone),
    }
}

/// Resolve each declared param to concrete text: the positional/glued argument
/// if supplied, else the declared default, else the empty string.
fn bind_values(snippet: &CompiledSnippet, args: &[String], remainder: &str) -> Values {
    let mut slots: Vec<Option<String>> = snippet.params.iter().map(|_| None).collect();
    for (slot, arg) in slots.iter_mut().zip(args) {
        *slot = Some(arg.clone());
    }
    if !remainder.is_empty() {
        let kind = if remainder.bytes().all(|b| b.is_ascii_digit()) {
            ParamKind::Int
        } else {
            ParamKind::Ident
        };
        for (param, slot) in snippet.params.iter().zip(&mut slots) {
            if param.kind == kind && slot.is_none() {
                *slot = Some(remainder.to_owned());
                break;
            }
        }
    }
    snippet
        .params
        .iter()
        .zip(slots)
        .map(|(param, slot)| {
            let value = slot.or_else(|| param.default.clone()).unwrap_or_default();
            (param.name.clone(), value)
        })
        .collect()
}

/// Concrete parameter values, by name.
type Values = BTreeMap<String, String>;

/// Substitute `${name}` markers with their values, leaving any other `${…}`
/// (tabstops like `${1:x}`) untouched.
fn subst(text: &str, values: &Values) -> String {
    substitute_markers(text, |name| {
        values
            .get(name)
            .cloned()
            .unwrap_or_else(|| format!("${{{name}}}"))
    })
}

/// Choose the primary piece: the one marked `primary`, else the `self`-targeted
/// one (the file you invoked from), else the first.
fn choose_primary(files: &[CompiledFilePiece]) -> usize {
    files
        .iter()
        .position(|f| f.primary)
        .or_else(|| {
            files
                .iter()
                .position(|f| matches!(f.target, FileTarget::SelfFile))
        })
        .unwrap_or(0)
}

/// Resolve a piece's target to a concrete path.
fn resolve_path(
    piece: &CompiledFilePiece,
    values: &Values,
    opts: &ScaffoldOptions,
) -> Result<PathBuf, ScaffoldError> {
    let current = || {
        opts.current_file
            .ok_or_else(|| ScaffoldError::NoCurrentFile {
                label: piece.label.clone(),
            })
    };
    match &piece.target {
        FileTarget::Path(template) => Ok(opts.root.join(subst(template, values))),
        FileTarget::SelfFile => current().map(Path::to_path_buf),
        // `.cpp` → the current file with its extension swapped to `cpp`.
        FileTarget::Sibling(ext) => Ok(current()?.with_extension(ext.trim_start_matches('.'))),
    }
}

/// Render one piece's content to text (params already resolved to concrete
/// values).
fn render_piece(
    engine: &Engine,
    language: &str,
    piece: &CompiledFilePiece,
    values: &Values,
    ctx: &ExpansionContext,
    mode: TabstopMode,
) -> Result<String, ScaffoldError> {
    match &piece.content {
        PieceContent::Body(body) => {
            let resolved = subst(body, values);
            let template = parse_template(&resolved).map_err(|source| ScaffoldError::Body {
                label: piece.label.clone(),
                source,
            })?;
            render_template(template, ctx, mode).map_err(|source| ScaffoldError::Render {
                label: piece.label.clone(),
                source,
            })
        }
        PieceContent::Expands(abbr) => {
            let resolved = subst(abbr, values);
            engine
                .expand(&resolved, language, ctx, mode)
                .map(|e| e.body)
                .map_err(|source| ScaffoldError::Expand {
                    label: piece.label.clone(),
                    abbr: resolved,
                    source: Box::new(source),
                })
        }
    }
}

/// Render a literal body template (no params left) through the normal pipeline,
/// so tabstop renumbering, indentation, and escaping match `expand`. Wrapped in
/// a throwaway single-snippet set rendered by a trivial abbreviation.
fn render_template(
    template: Template,
    ctx: &ExpansionContext,
    mode: TabstopMode,
) -> Result<String, RenderError> {
    let snippet = CompiledSnippet {
        trigger: "s".to_owned(),
        description: None,
        params: Vec::new(),
        template,
        scope: None,
        variants: Vec::new(),
        composite: None,
        requires: Vec::new(),
        multifile: None,
    };
    let set = SnippetSet::from_compiled([snippet]);
    let ast = Abbreviation(vec![AbbrNode {
        word: "s".to_owned(),
        args: Vec::new(),
        repeat: Repeat::Count(1),
        children: Vec::new(),
    }]);
    render(&ast, &set, ctx, mode).map(|e| e.body)
}

#[cfg(test)]
// Expected values like `${1:x}` are LSP snippet syntax, not format strings.
#[allow(clippy::literal_string_with_formatting_args)]
mod tests {
    use super::*;
    use crate::config::loader::{CompiledMultiFile, PieceContent};
    use crate::config::schema::{ParamDef, ParamKind};

    fn param(name: &str, kind: ParamKind, default: Option<&str>) -> ParamDef {
        ParamDef {
            name: name.to_owned(),
            kind,
            default: default.map(ToOwned::to_owned),
        }
    }

    fn multifile_snippet(params: Vec<ParamDef>, files: Vec<CompiledFilePiece>) -> CompiledSnippet {
        CompiledSnippet {
            trigger: "mf".to_owned(),
            description: None,
            params,
            template: Template { lines: Vec::new() },
            scope: None,
            variants: Vec::new(),
            composite: None,
            multifile: Some(CompiledMultiFile { files }),
        }
    }

    fn piece(label: &str, target: FileTarget, primary: bool) -> CompiledFilePiece {
        CompiledFilePiece {
            label: label.to_owned(),
            target,
            content: PieceContent::Body("x".to_owned()),
            on_exists: OnExists::default(),
            into: None,
            on_create: None,
            primary,
        }
    }

    fn values(pairs: &[(&str, &str)]) -> Values {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn bind_values_uses_args_then_glue_then_default() {
        let snip = multifile_snippet(
            vec![
                param("app", ParamKind::Ident, Some("app")),
                param("n", ParamKind::Int, Some("1")),
            ],
            vec![],
        );
        // Explicit args bind positionally.
        assert_eq!(
            bind_values(&snip, &["blog".to_owned(), "3".to_owned()], ""),
            values(&[("app", "blog"), ("n", "3")])
        );
        // No args: declared defaults.
        assert_eq!(
            bind_values(&snip, &[], ""),
            values(&[("app", "app"), ("n", "1")])
        );
        // A glued int binds the first `int` param, leaving the ident defaulted.
        assert_eq!(
            bind_values(&snip, &[], "7"),
            values(&[("app", "app"), ("n", "7")])
        );
    }

    #[test]
    fn subst_replaces_params_and_keeps_tabstops() {
        let v = values(&[("app", "blog")]);
        assert_eq!(subst("${app}/models.py", &v), "blog/models.py");
        // `${1:x}` and `$0` are not params and must survive untouched.
        assert_eq!(
            subst("def ${app}():\n\t${1:pass}$0", &v),
            "def blog():\n\t${1:pass}$0"
        );
    }

    #[test]
    fn choose_primary_prefers_flag_then_self_then_first() {
        let flagged = [
            piece("a", FileTarget::SelfFile, false),
            piece("b", FileTarget::Path("x".to_owned()), true),
        ];
        assert_eq!(choose_primary(&flagged), 1);

        let self_targeted = [
            piece("a", FileTarget::Path("x".to_owned()), false),
            piece("b", FileTarget::SelfFile, false),
        ];
        assert_eq!(choose_primary(&self_targeted), 1);

        let neither = [
            piece("a", FileTarget::Path("x".to_owned()), false),
            piece("b", FileTarget::Path("y".to_owned()), false),
        ];
        assert_eq!(choose_primary(&neither), 0);
    }

    #[test]
    fn resolve_path_templates_siblings_and_self() {
        let root = Path::new("/proj");
        let cur = Path::new("/proj/src/widget.hpp");
        let v = values(&[("app", "blog")]);

        let opts_no_file = ScaffoldOptions {
            root,
            current_file: None,
        };
        let path_piece = piece("m", FileTarget::Path("${app}/models.py".to_owned()), false);
        assert_eq!(
            resolve_path(&path_piece, &v, &opts_no_file).expect("path target resolves"),
            Path::new("/proj/blog/models.py")
        );

        let opts = ScaffoldOptions {
            root,
            current_file: Some(cur),
        };
        let self_piece = piece("s", FileTarget::SelfFile, false);
        assert_eq!(
            resolve_path(&self_piece, &v, &opts).expect("self target resolves"),
            cur
        );
        let sib = piece("d", FileTarget::Sibling(".cpp".to_owned()), false);
        assert_eq!(
            resolve_path(&sib, &v, &opts).expect("sibling target resolves"),
            Path::new("/proj/src/widget.cpp")
        );

        // A self/sibling target with no current file is an error.
        assert!(matches!(
            resolve_path(&self_piece, &v, &opts_no_file),
            Err(ScaffoldError::NoCurrentFile { .. })
        ));
    }

    #[test]
    fn standalone_node_rejects_chains() {
        let parse = |a: &str| crate::abbr::parse(a).expect("fixture parses");
        assert!(standalone_node(&parse("djfeature:blog")).is_ok());
        assert!(matches!(
            standalone_node(&parse("a>b")),
            Err(ScaffoldError::NotStandalone)
        ));
        assert!(matches!(
            standalone_node(&parse("a+b")),
            Err(ScaffoldError::NotStandalone)
        ));
        assert!(matches!(
            standalone_node(&parse("a*2")),
            Err(ScaffoldError::NotStandalone)
        ));
    }
}
