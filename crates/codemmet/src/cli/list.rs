//! `codemmet list`: enumerate a language's triggers.

use std::path::Path;
use std::process::ExitCode;

use clap::Args;

use super::{config_failure, load_registry};

/// Arguments for `codemmet list`.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Language id, file extension, or LSP language id.
    #[arg(long, value_name = "ID")]
    lang: String,

    /// Print a JSON array of `{"trigger", "description"}` objects.
    #[arg(long)]
    json: bool,
}

/// Run `codemmet list` with `user_dir` as the snippet override directory.
pub fn run(user_dir: Option<&Path>, args: &ListArgs) -> ExitCode {
    let registry = match load_registry(user_dir) {
        Ok(registry) => registry,
        Err(err) => return config_failure(&err),
    };
    let Some(lang) = registry.language(&args.lang) else {
        return config_failure(&format!("unknown language `{}`", args.lang));
    };

    if args.json {
        let items: Vec<serde_json::Value> = lang
            .set
            .iter()
            .map(|snippet| {
                serde_json::json!({
                    "trigger": snippet.trigger,
                    "description": snippet.description,
                })
            })
            .collect();
        println!("{}", serde_json::Value::Array(items));
    } else {
        let width = lang
            .set
            .iter()
            .map(|snippet| snippet.trigger.len())
            .max()
            .unwrap_or(0);
        for snippet in lang.set.iter() {
            let description = snippet.description.as_deref().unwrap_or("");
            println!("{:<width$}\t{description}", snippet.trigger);
        }
    }
    ExitCode::SUCCESS
}
