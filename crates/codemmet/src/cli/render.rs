//! `codemmet render`: a `.codemmet` source file in, rendered code out.
//!
//! File mode (see `FILE_MODE_PLAN.md`) folds over the classified lines of a
//! `.codemmet` file, expanding each abbreviation at the end of the output
//! rendered so far, and writes stdout only when every line succeeded — a
//! mid-file error emits zero bytes, so `codemmet … >> main.rs` never leaves
//! a half-appended file. Diagnostics carry the source name and 1-based
//! `.codemmet` line number.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Args;
use codemmet_core::Engine;
use codemmet_core::context::{
    Collection, ContextProvider, DocSnapshot, IndentUnit, NullContextProvider,
};
use codemmet_core::engine::ExpandError;
use codemmet_core::filefmt::{self, Line};
use codemmet_core::lang::registry::Registry;
use codemmet_core::snippet::render::TabstopMode;

use super::{
    EXIT_CONFIG, EXIT_PARSE, EXIT_RENDER, Sources, Tabstops, config_failure, load_registry,
    resolve_indent_unit,
};

/// Arguments for `codemmet render`.
#[derive(Debug, Args)]
pub struct RenderArgs {
    /// Language id, file extension, or LSP language id. A `#lang` directive
    /// overrides this; the file name's inner extension
    /// (`NAME.EXT.codemmet`) is the fallback.
    #[arg(long, value_name = "ID")]
    lang: Option<String>,

    /// Literal indent unit: a single tab or a run of spaces
    /// (default: the language profile's indent).
    #[arg(long, value_name = "STR")]
    indent: Option<String>,

    /// Tabstop handling. File-mode output is always plain code, so only
    /// `strip` is accepted; `keep` is an error.
    #[arg(long, value_enum)]
    tabstops: Option<Tabstops>,

    /// Seed a collection for data-driven repeats, as `NAME=v1,v2,…`
    /// (repeatable). A same-name `#repeat` directive overrides it.
    #[arg(long, value_name = "NAME=v1,v2,…")]
    repeat: Vec<String>,

    /// The `.codemmet` source file; omitted or `-` reads stdin.
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
}

impl RenderArgs {
    /// Arguments for the top-level positional spelling
    /// (`codemmet main.rs.codemmet`).
    pub const fn for_file(file: PathBuf) -> Self {
        Self {
            lang: None,
            indent: None,
            tabstops: None,
            repeat: Vec::new(),
            file: Some(file),
        }
    }
}

/// A failure at one line of the source file.
struct Failure {
    /// 1-based line number in the `.codemmet` source.
    line: usize,
    message: String,
    /// Process exit code, per the `PLAN.md` conventions.
    code: u8,
}

/// A per-line error before it is pinned to a line number.
type StepError = (String, u8);

/// Run `codemmet render`.
pub fn run(sources: &Sources, args: &RenderArgs) -> ExitCode {
    if args.tabstops == Some(Tabstops::Keep) {
        return config_failure(
            &"--tabstops keep is not supported in file mode: the output is plain code \
             (tabstops are always stripped)",
        );
    }
    let registry = match load_registry(sources) {
        Ok(registry) => registry,
        Err(err) => return config_failure(&err),
    };
    let seeds = match super::expand::parse_repeat(&args.repeat) {
        Ok(seeds) => seeds,
        Err(msg) => return config_failure(&msg),
    };
    let (source_name, text) = match read_source(args.file.as_deref()) {
        Ok(source) => source,
        Err(msg) => return config_failure(&msg),
    };
    match render_file(registry, &seeds, args, &text) {
        Ok(output) => {
            // The whole render succeeded: only now does anything reach
            // stdout (the atomicity contract for `>>` appends).
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("error: {source_name}:{}: {}", failure.line, failure.message);
            ExitCode::from(failure.code)
        }
    }
}

/// Read the source text: a file path, or stdin for `-`/omitted. Returns the
/// diagnostic source name alongside the text.
fn read_source(file: Option<&Path>) -> Result<(String, String), String> {
    match file {
        Some(path) if path.as_os_str() != "-" => {
            let text = std::fs::read_to_string(path)
                .map_err(|err| format!("cannot read `{}`: {err}", path.display()))?;
            Ok((path.display().to_string(), text))
        }
        _ => {
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .map_err(|err| format!("cannot read stdin: {err}"))?;
            Ok(("<stdin>".to_string(), text))
        }
    }
}

/// Render the whole file to memory, failing fast at the first bad line.
fn render_file(
    registry: Registry,
    seeds: &BTreeMap<String, Collection>,
    args: &RenderArgs,
    text: &str,
) -> Result<String, Failure> {
    let mut state = FileState::new(registry, seeds, args);
    for (line_no, classified) in filefmt::lines(text) {
        let step = match classified {
            Err(err) => Err((err.to_string(), EXIT_PARSE)),
            Ok(Line::Comment) => Ok(()),
            Ok(Line::Blank) => {
                state.blank();
                Ok(())
            }
            Ok(Line::Directive { name, value }) => state.directive(name, value),
            Ok(Line::Abbrev { indent, abbr }) => state.abbrev(indent, abbr),
        };
        if let Err((message, code)) = step {
            return Err(Failure {
                line: line_no,
                message,
                code,
            });
        }
    }
    Ok(state.buffer)
}

/// The language, indentation, and engine resolved at the first abbreviation.
struct LangState {
    /// The resolved language id (directive > `--lang` > filename inference).
    lang: String,
    /// The indent unit (`--indent` or the profile default).
    indent_unit: IndentUnit,
    engine: Engine,
}

/// The fold state of one file render.
struct FileState<'a> {
    args: &'a RenderArgs,
    /// CLI `--repeat` seed collections (below `#repeat` in precedence).
    seeds: &'a BTreeMap<String, Collection>,
    /// Owned until the first abbreviation moves it into the engine.
    registry: Option<Registry>,
    /// Buffer/syntax awareness for progressive context.
    provider: Box<dyn ContextProvider>,
    /// The `#lang` directive's value, if seen.
    lang_directive: Option<String>,
    /// `#repeat` collections, effective from their line down.
    directive_collections: BTreeMap<String, Collection>,
    /// Set at the first abbreviation line.
    lang_state: Option<LangState>,
    /// Everything rendered so far — the progressive-context document.
    buffer: String,
    /// Whether any output line exists yet (leading blanks are dropped).
    emitted: bool,
}

impl<'a> FileState<'a> {
    fn new(
        registry: Registry,
        seeds: &'a BTreeMap<String, Collection>,
        args: &'a RenderArgs,
    ) -> Self {
        Self {
            args,
            seeds,
            registry: Some(registry),
            provider: Box::new(NullContextProvider),
            lang_directive: None,
            directive_collections: BTreeMap::new(),
            lang_state: None,
            buffer: String::new(),
            emitted: false,
        }
    }

    /// A blank source line is a blank output line — except before any
    /// output exists, so a blank separating the directive header from the
    /// body does not become a leading blank in the render.
    fn blank(&mut self) {
        if self.emitted {
            self.buffer.push('\n');
        }
    }

    /// Apply a directive line.
    fn directive(&mut self, name: &str, value: &str) -> Result<(), StepError> {
        match name {
            "lang" => self.lang_directive(value),
            "repeat" => self.repeat_directive(value),
            other => Err((format!("unsupported directive `#{other}`"), EXIT_PARSE)),
        }
    }

    /// `#lang <id>`: once per file, before the first abbreviation.
    fn lang_directive(&mut self, value: &str) -> Result<(), StepError> {
        if self.lang_state.is_some() {
            return Err((
                "`#lang` must appear before the first abbreviation".to_string(),
                EXIT_PARSE,
            ));
        }
        if self.lang_directive.is_some() {
            return Err((
                "duplicate `#lang` (a file renders in one language)".to_string(),
                EXIT_PARSE,
            ));
        }
        if value.is_empty() {
            return Err((
                "`#lang` needs a value, e.g. `#lang rust`".to_string(),
                EXIT_PARSE,
            ));
        }
        let registry = self
            .registry
            .as_ref()
            .expect("registry is owned until the first abbreviation");
        if registry.language(value).is_none() {
            return Err((format!("unknown language `{value}`"), EXIT_CONFIG));
        }
        self.lang_directive = Some(value.to_string());
        Ok(())
    }

    /// `#repeat NAME=v1,v2,…`: define/shadow a collection from here down.
    fn repeat_directive(&mut self, value: &str) -> Result<(), StepError> {
        if value.is_empty() {
            return Err((
                "`#repeat` needs a value, e.g. `#repeat Msg=Quit,Move`".to_string(),
                EXIT_PARSE,
            ));
        }
        let (name, collection) =
            Collection::parse_spec(value).map_err(|err| (format!("#repeat {err}"), EXIT_PARSE))?;
        self.directive_collections.insert(name, collection);
        Ok(())
    }

    /// Expand one abbreviation at the end of the buffer and append it.
    fn abbrev(&mut self, indent: &str, abbr: &str) -> Result<(), StepError> {
        self.ensure_lang_state()?;
        let state = self
            .lang_state
            .as_ref()
            .expect("just ensured the language state");
        let doc = DocSnapshot {
            language: state.lang.clone(),
            text: self.buffer.clone(),
        };
        // The virtual cursor sits at the end of everything rendered so far.
        let mut ctx = self.provider.context_at(&doc, doc.text.len());
        ctx.base_indent = indent.to_string();
        ctx.indent_unit = state.indent_unit.clone();
        // Collection precedence: buffer-extracted < --repeat < #repeat.
        ctx.collections.extend(self.seeds.clone());
        ctx.collections.extend(self.directive_collections.clone());
        let expansion = state
            .engine
            .expand(abbr, &state.lang, &ctx, TabstopMode::Strip)
            .map_err(|err| (format!("cannot expand `{abbr}`: {err}"), exit_code(&err)))?;
        push_block(&mut self.buffer, indent, &expansion.body);
        self.emitted = true;
        Ok(())
    }

    /// Resolve language, indent unit, and engine at the first abbreviation:
    /// `#lang` directive > `--lang` flag > `NAME.EXT.codemmet` inference.
    fn ensure_lang_state(&mut self) -> Result<(), StepError> {
        if self.lang_state.is_some() {
            return Ok(());
        }
        let lang = self
            .lang_directive
            .clone()
            .or_else(|| self.args.lang.clone())
            .or_else(|| infer_lang(self.args.file.as_deref()))
            .ok_or_else(|| {
                (
                    "cannot resolve a language: add a `#lang` directive, pass --lang, \
                     or name the file `NAME.EXT.codemmet`"
                        .to_string(),
                    EXIT_CONFIG,
                )
            })?;
        let registry = self
            .registry
            .take()
            .expect("registry is owned until the first abbreviation");
        let indent_unit = resolve_indent_unit(&registry, &lang, self.args.indent.as_deref())
            .map_err(|msg| (msg, EXIT_CONFIG))?;
        self.lang_state = Some(LangState {
            lang,
            indent_unit,
            engine: Engine::with_registry(registry),
        });
        Ok(())
    }
}

/// Append one expansion to the buffer: the base indent prefixes the first
/// line (the renderer only indents continuation lines — in an editor the
/// cursor already sits after the indent), and every line is right-trimmed
/// so a stripped `$0` never leaves a whitespace-only line.
fn push_block(buffer: &mut String, indent: &str, body: &str) {
    for (idx, raw) in body.lines().enumerate() {
        if idx == 0 {
            let first = format!("{indent}{raw}");
            buffer.push_str(first.trim_end());
        } else {
            buffer.push_str(raw.trim_end());
        }
        buffer.push('\n');
    }
}

/// The exit code an expansion failure maps to (the `expand` conventions).
const fn exit_code(err: &ExpandError) -> u8 {
    match err {
        ExpandError::Parse(_) => EXIT_PARSE,
        ExpandError::Render(_) => EXIT_RENDER,
        ExpandError::UnknownLanguage(_) => EXIT_CONFIG,
    }
}

/// The language implied by a `NAME.EXT.codemmet` file name: the inner
/// extension, resolved through the usual extension→profile mapping later.
fn infer_lang(file: Option<&Path>) -> Option<String> {
    let name = file?.file_name()?.to_str()?;
    let inner = name.strip_suffix(".codemmet")?;
    let ext = Path::new(inner).extension()?.to_str()?;
    Some(ext.to_string())
}
