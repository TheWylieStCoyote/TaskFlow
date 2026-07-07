//! `codemmet check`: validate the user snippet override directory.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use codemmet_core::lang::profile::PROFILES;

use super::{Sources, config_failure, load_registry};

/// Run `codemmet check` against the resolved override directories.
///
/// A missing directory is not an error: builtins are compiled in and there
/// is simply nothing to validate.
pub fn run(sources: &Sources) -> ExitCode {
    let labeled = sources.labeled_dirs();
    // Preserve the friendly single-directory notes when the only tier is the
    // user directory (no path-mapped or project packs to also consider).
    if labeled.len() <= 1 {
        match sources.user_dir() {
            None => {
                println!("no user snippets directory configured; builtins only");
                return ExitCode::SUCCESS;
            }
            Some(dir) if !dir.exists() => {
                println!("no user snippets at {}", dir.display());
                return ExitCode::SUCCESS;
            }
            Some(_) => {}
        }
    }
    match load_registry(sources) {
        Ok(registry) => {
            for (_, dir) in &labeled {
                if dir.exists() {
                    warn_misnamed_packs(dir);
                }
            }
            let languages = registry.languages().count();
            let triggers: usize = registry.languages().map(|lang| lang.set.len()).sum();
            println!("ok: {languages} languages, {triggers} triggers");
            for (label, dir) in &labeled {
                if dir.exists() {
                    println!("  {label} snippets: {}", dir.display());
                }
            }
            ExitCode::SUCCESS
        }
        Err(err) => config_failure(&err),
    }
}

/// Warn (stderr) about `*.toml` files in `dir` whose stem is not a known
/// profile id: the loader only looks up `<profile-id>.toml`, so a misnamed
/// pack (`rs.toml`, `Rust.toml`) silently never loads.
fn warn_misnamed_packs(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut misnamed: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().is_some_and(|ext| ext == "toml")
                && !path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| PROFILES.iter().any(|profile| profile.id == stem))
        })
        .collect();
    misnamed.sort();
    if misnamed.is_empty() {
        return;
    }
    let mut expected: Vec<String> = PROFILES
        .iter()
        .map(|profile| format!("{}.toml", profile.id))
        .collect();
    expected.sort_unstable();
    let expected = expected.join(", ");
    for path in misnamed {
        eprintln!(
            "warning: {} does not match any language pack name (expected one of: {expected})",
            path.display()
        );
    }
}
