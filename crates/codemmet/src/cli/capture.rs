//! `codemmet capture`: generalize example code into a reusable snippet and
//! write it into a snippet pack.

use std::io::Read as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use codemmet_core::config::writer::{SnippetEntry, upsert_snippet};
use codemmet_core::lang::profile;
use codemmet_core::lang::registry::resolve_capture_dir;
use codemmet_core::snippet::capture::{CaptureOptions, capture};

use super::{Sources, config_failure};

/// Arguments for `codemmet capture`.
#[derive(Debug, Args)]
pub struct CaptureArgs {
    /// Language id, file extension, or LSP language id.
    #[arg(long, value_name = "ID")]
    lang: String,

    /// The trigger name to register the captured snippet under.
    #[arg(long, value_name = "NAME")]
    name: String,

    /// Optional description shown by `list` and editor completion.
    #[arg(long, value_name = "TEXT")]
    description: Option<String>,

    /// Force an identifier to become a `${param}`, even if it occurs once.
    /// Repeatable.
    #[arg(long = "param", value_name = "IDENT")]
    params: Vec<String>,

    /// Keep an identifier literal, even if it repeats. Repeatable.
    #[arg(long = "literal", value_name = "IDENT")]
    literals: Vec<String>,

    /// Write to the user snippet directory instead of the project pack.
    #[arg(long)]
    user: bool,

    /// Overwrite an existing trigger of the same name.
    #[arg(long)]
    force: bool,

    /// The code to capture; omitted or `-` reads it from stdin.
    #[arg(value_name = "CODE")]
    code: Option<String>,
}

/// Run `codemmet capture`.
pub fn run(sources: Sources, args: &CaptureArgs) -> ExitCode {
    // Canonicalize the language (accepts extensions / LSP ids) so the pack file
    // name and `meta.language` are the profile id.
    let Some(profile) = profile::find(&args.lang) else {
        return config_failure(&format!("unknown language `{}`", args.lang));
    };
    let lang = profile.id;

    let code = match read_code(args.code.as_deref()) {
        Ok(code) => code,
        Err(msg) => return config_failure(&msg),
    };
    if code.trim().is_empty() {
        return config_failure(&"nothing to capture: the code is empty");
    }

    let target_dir = match resolve_target(sources, args.user) {
        Ok(dir) => dir,
        Err(msg) => return config_failure(&msg),
    };

    let options = CaptureOptions {
        force_params: args.params.clone(),
        force_literals: args.literals.clone(),
    };
    let captured = capture(lang, &code, &options);
    let entry = SnippetEntry {
        trigger: args.name.clone(),
        description: args.description.clone(),
        params: captured.params,
        body: captured.body,
    };

    match upsert_snippet(&target_dir, lang, &entry, args.force) {
        Ok(path) => {
            let count = entry.params.len();
            let names: Vec<&str> = entry.params.iter().map(|p| p.name.as_str()).collect();
            let params = if count == 0 {
                "no parameters".to_owned()
            } else {
                format!("{count} parameter(s): {}", names.join(", "))
            };
            println!(
                "captured `{}` ({params}) into {}",
                args.name,
                path.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => config_failure(&err),
    }
}

/// Resolve the directory to write the pack into.
///
/// `--user` writes to the user override directory (`--config` or the default);
/// otherwise the project pack (`.codemmet/snippets` at or above the working
/// directory, else a fresh one under the project root) is used.
fn resolve_target(sources: Sources, user: bool) -> Result<PathBuf, String> {
    if user {
        return sources
            .user
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| {
                "no user snippets directory: pass --config <DIR> or build with the `dirs` feature"
                    .to_owned()
            });
    }
    let cwd = std::env::current_dir()
        .map_err(|err| format!("cannot determine the working directory: {err}"))?;
    Ok(resolve_capture_dir(&cwd))
}

/// Read the code to capture; `None` or `-` reads all of stdin.
fn read_code(arg: Option<&str>) -> Result<String, String> {
    match arg {
        Some(code) if code != "-" => Ok(code.to_owned()),
        _ => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|err| format!("cannot read code from stdin: {err}"))?;
            Ok(buf)
        }
    }
}
