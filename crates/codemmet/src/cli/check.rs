//! `codemmet check`: validate the user snippet override directory.

use std::path::Path;
use std::process::ExitCode;

use super::{config_failure, load_registry};

/// Run `codemmet check` against `user_dir` (already defaulted by the caller).
///
/// A missing directory is not an error: builtins are compiled in and there
/// is simply nothing to validate.
pub fn run(user_dir: Option<&Path>) -> ExitCode {
    let Some(dir) = user_dir else {
        println!("no user snippets directory configured; builtins only");
        return ExitCode::SUCCESS;
    };
    if !dir.exists() {
        println!("no user snippets at {}", dir.display());
        return ExitCode::SUCCESS;
    }
    match load_registry(Some(dir)) {
        Ok(registry) => {
            let languages = registry.languages().count();
            let triggers: usize = registry.languages().map(|lang| lang.set.len()).sum();
            println!(
                "ok: {languages} languages, {triggers} triggers (user snippets: {})",
                dir.display()
            );
            ExitCode::SUCCESS
        }
        Err(err) => config_failure(&err),
    }
}
