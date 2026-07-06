//! Bakes the current git short hash into the long version string.
//!
//! Sets `CODEMMET_LONG_VERSION` to `"<pkg-version> (<short-hash>)"`, or just
//! the package version when the hash is unavailable (no git, not a checkout,
//! e.g. a crates.io build). Packagers can pin the hash via the
//! `CODEMMET_GIT_HASH` environment variable.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let pkg = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_owned());
    let long = match git_short_hash() {
        // A `-dirty` suffix flags a build made with uncommitted (tracked)
        // changes, so the hash isn't mistaken for that exact commit.
        Some(hash) if is_dirty() => format!("{pkg} ({hash}-dirty)"),
        Some(hash) => format!("{pkg} ({hash})"),
        None => pkg,
    };
    println!("cargo:rustc-env=CODEMMET_LONG_VERSION={long}");
    emit_rerun();
}

/// Whether the working tree has uncommitted changes to tracked files.
///
/// Untracked files are ignored (matching `git describe --dirty`), so scratch
/// files don't flip the marker.
fn is_dirty() -> bool {
    Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .is_some_and(|out| !out.stdout.is_empty())
}

/// The short commit hash, or `None` when git or a repository is unavailable.
fn git_short_hash() -> Option<String> {
    // An explicit override wins (packagers, reproducible builds).
    if let Ok(hash) = env::var("CODEMMET_GIT_HASH") {
        let hash = hash.trim().to_owned();
        if !hash.is_empty() {
            return Some(hash);
        }
    }
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let hash = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    if hash.is_empty() { None } else { Some(hash) }
}

/// Re-run the build script when the checked-out commit (or override) changes.
fn emit_rerun() {
    println!("cargo:rerun-if-env-changed=CODEMMET_GIT_HASH");
    let Some(git_dir) = git_dir() else {
        return;
    };
    let head = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head.display());
    // `index` changes on commit / checkout / staging, refreshing the hash and
    // the dirty marker. Unstaged edits between builds may not be reflected
    // until the next commit, checkout, `git add`, or clean rebuild.
    println!("cargo:rerun-if-changed={}", git_dir.join("index").display());
    // A branch checkout points HEAD at a ref file; watch that too so new
    // commits on the same branch refresh the hash.
    if let Ok(contents) = fs::read_to_string(&head)
        && let Some(reference) = contents.strip_prefix("ref:").map(str::trim)
    {
        println!(
            "cargo:rerun-if-changed={}",
            git_dir.join(reference).display()
        );
    }
}

/// The repository's `.git` directory, if this is a checkout.
fn git_dir() -> Option<PathBuf> {
    // Build scripts run with `CARGO_MANIFEST_DIR` = this crate's directory;
    // the workspace root is two levels up.
    let manifest = env::var("CARGO_MANIFEST_DIR").ok()?;
    let git = PathBuf::from(manifest).join("..").join("..").join(".git");
    git.exists().then_some(git)
}
