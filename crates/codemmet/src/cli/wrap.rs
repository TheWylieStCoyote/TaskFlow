//! `codemmet wrap`: wrap a selection (stdin) in an expanded abbreviation.

use std::io::Read as _;
use std::process::ExitCode;

use clap::Args;
use codemmet_core::Engine;
use codemmet_core::context::ExpansionContext;
use codemmet_core::engine::ExpandError;

use super::{
    EXIT_PARSE, EXIT_RENDER, Sources, Tabstops, config_failure, load_registry, resolve_indent_unit,
};

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
pub fn run(sources: &Sources, args: &WrapArgs) -> ExitCode {
    let registry = match load_registry(sources) {
        Ok(registry) => registry,
        Err(err) => return config_failure(&err),
    };

    let indent_unit = match resolve_indent_unit(&registry, &args.lang, args.indent.as_deref()) {
        Ok(unit) => unit,
        Err(msg) => return config_failure(&msg),
    };

    let mut selection = String::new();
    if let Err(err) = std::io::stdin().read_to_string(&mut selection) {
        return config_failure(&format!("cannot read selection from stdin: {err}"));
    }
    let selection = selection.trim_end_matches(['\r', '\n']);

    let engine = Engine::with_registry(registry);
    let ctx = ExpansionContext::plain(args.base_indent.clone(), indent_unit);
    match engine.expand_wrap(
        &args.abbr,
        &args.lang,
        &ctx,
        args.tabstops.into(),
        selection,
    ) {
        Ok(expansion) => {
            println!("{}", expansion.body);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: cannot wrap with `{}`: {err}", args.abbr);
            let code = match err {
                ExpandError::Parse(_) => EXIT_PARSE,
                ExpandError::Render(_) => EXIT_RENDER,
                ExpandError::UnknownLanguage(_) => super::EXIT_CONFIG,
            };
            ExitCode::from(code)
        }
    }
}
