//! `codemmet wrap`: wrap a selection (stdin) in an expanded abbreviation.

use std::io::Read as _;
use std::path::Path;
use std::process::ExitCode;

use clap::{Args, ValueEnum};
use codemmet_core::Engine;
use codemmet_core::context::{ExpansionContext, IndentUnit};
use codemmet_core::engine::ExpandError;

use super::{EXIT_PARSE, EXIT_RENDER, config_failure, load_registry};
use crate::wrap::{WrapError, strip_snippet_markers, wrap};

/// Tabstop handling for the wrapped output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Tabstops {
    /// Emit LSP snippet syntax (`${1:placeholder}`, `$0`).
    Keep,
    /// Emit plain text; placeholders become their text, markers vanish.
    Strip,
}

/// Arguments for `codemmet wrap`.
#[derive(Debug, Args)]
pub struct WrapArgs {
    /// Language id, file extension, or LSP language id.
    #[arg(long, value_name = "ID")]
    lang: String,

    /// The abbreviation to wrap the selection with.
    #[arg(long, value_name = "ABBR")]
    abbr: String,

    /// Tabstop handling.
    #[arg(long, value_enum, default_value = "strip")]
    tabstops: Tabstops,

    /// Literal indent unit: a single tab or a run of spaces
    /// (default: the language profile's indent).
    #[arg(long, value_name = "STR")]
    indent: Option<String>,

    /// Prefix prepended to every continuation line.
    #[arg(long, value_name = "STR", default_value = "")]
    base_indent: String,
}

/// Run `codemmet wrap` with `user_dir` as the snippet override directory.
pub fn run(user_dir: Option<&Path>, args: &WrapArgs) -> ExitCode {
    let registry = match load_registry(user_dir) {
        Ok(registry) => registry,
        Err(err) => return config_failure(&err),
    };

    let indent_unit = {
        let Some(lang) = registry.language(&args.lang) else {
            return config_failure(&format!("unknown language `{}`", args.lang));
        };
        match args.indent.as_deref() {
            Some(text) => match parse_indent(text) {
                Ok(unit) => unit,
                Err(msg) => return config_failure(&msg),
            },
            None => lang.profile.default_indent.clone(),
        }
    };

    let mut selection = String::new();
    if let Err(err) = std::io::stdin().read_to_string(&mut selection) {
        return config_failure(&format!("cannot read selection from stdin: {err}"));
    }
    let selection = selection.trim_end_matches(['\r', '\n']);

    let engine = Engine::with_registry(registry);
    let ctx = ExpansionContext::plain(args.base_indent.clone(), indent_unit);
    match wrap(&engine, &args.lang, &args.abbr, selection, &ctx) {
        Ok(snippet) => {
            match args.tabstops {
                Tabstops::Keep => println!("{snippet}"),
                Tabstops::Strip => println!("{}", strip_snippet_markers(&snippet)),
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: cannot wrap with `{}`: {err}", args.abbr);
            let code = match err {
                WrapError::Expand(ExpandError::Parse(_)) => EXIT_PARSE,
                WrapError::Expand(ExpandError::Render(_)) | WrapError::NoCursor => EXIT_RENDER,
                WrapError::Expand(ExpandError::UnknownLanguage(_)) => super::EXIT_CONFIG,
            };
            ExitCode::from(code)
        }
    }
}

/// Parse a `--indent` value: `"\t"` or a non-empty run of spaces.
fn parse_indent(text: &str) -> Result<IndentUnit, String> {
    if text == "\t" {
        return Ok(IndentUnit::Tab);
    }
    if !text.is_empty() && text.bytes().all(|b| b == b' ') {
        return u8::try_from(text.len())
            .map(IndentUnit::Spaces)
            .map_err(|_| format!("--indent is too wide: {} spaces (max 255)", text.len()));
    }
    Err(format!(
        "invalid --indent {text:?}: expected a single tab or a run of spaces"
    ))
}
