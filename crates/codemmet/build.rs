//! Bakes git and toolchain metadata into the binary's version strings.
//!
//! Sets `CODEMMET_LONG_VERSION` to `"<pkg-version> (<short-hash>)"` (just the
//! package version when the hash is unavailable — no git, not a checkout,
//! e.g. a crates.io build), plus the detailed `-V` fields:
//! `CODEMMET_GIT_COMMIT` (full hash), `CODEMMET_GIT_BRANCH`,
//! `CODEMMET_GIT_DIRTY` (`clean`/`dirty`/`unknown`), `CODEMMET_BUILD_PROFILE`,
//! `CODEMMET_BUILD_TARGET`, `CODEMMET_RUSTC`, and `CODEMMET_FEATURES`.
//! Packagers can pin the hash via the `CODEMMET_GIT_HASH` environment
//! variable and the branch via `CODEMMET_GIT_BRANCH`; unknowns degrade to
//! `"unknown"` rather than failing the build.

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

    // Detailed `-V` fields. Every value degrades to "unknown" so builds
    // outside a checkout (crates.io, tarballs) stay reproducible and quiet.
    let unknown = || "unknown".to_owned();
    println!(
        "cargo:rustc-env=CODEMMET_GIT_COMMIT={}",
        git_full_hash().unwrap_or_else(unknown)
    );
    println!(
        "cargo:rustc-env=CODEMMET_GIT_BRANCH={}",
        git_branch().unwrap_or_else(unknown)
    );
    let dirty = if git_full_hash().is_none() {
        "unknown"
    } else if is_dirty() {
        "dirty"
    } else {
        "clean"
    };
    println!("cargo:rustc-env=CODEMMET_GIT_DIRTY={dirty}");
    println!(
        "cargo:rustc-env=CODEMMET_BUILD_PROFILE={}",
        env::var("PROFILE").unwrap_or_else(|_| unknown())
    );
    println!(
        "cargo:rustc-env=CODEMMET_BUILD_TARGET={}",
        env::var("TARGET").unwrap_or_else(|_| unknown())
    );
    println!(
        "cargo:rustc-env=CODEMMET_RUSTC={}",
        rustc_version().unwrap_or_else(unknown)
    );
    println!("cargo:rustc-env=CODEMMET_FEATURES={}", features());
    emit_rerun();
}

/// The full commit hash, or `None` outside a checkout. The
/// `CODEMMET_GIT_HASH` override wins here too (it may be short — packagers
/// pin what they have).
fn git_full_hash() -> Option<String> {
    if let Ok(hash) = env::var("CODEMMET_GIT_HASH") {
        let hash = hash.trim().to_owned();
        if !hash.is_empty() {
            return Some(hash);
        }
    }
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let hash = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    if hash.is_empty() { None } else { Some(hash) }
}

/// The checked-out branch name; `"detached"` for a detached HEAD; `None`
/// outside a checkout. The `CODEMMET_GIT_BRANCH` override wins.
fn git_branch() -> Option<String> {
    if let Ok(branch) = env::var("CODEMMET_GIT_BRANCH") {
        let branch = branch.trim().to_owned();
        if !branch.is_empty() {
            return Some(branch);
        }
    }
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    match branch.as_str() {
        "" => None,
        "HEAD" => Some("detached".to_owned()),
        _ => Some(branch),
    }
}

/// The compiling rustc's version line (`rustc 1.xx.x (…)`).
fn rustc_version() -> Option<String> {
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let output = Command::new(rustc).arg("-V").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    if line.is_empty() { None } else { Some(line) }
}

/// The crate features enabled for this build, comma-separated (`none` when
/// empty), from Cargo's `CARGO_FEATURE_*` environment.
fn features() -> String {
    let mut names: Vec<String> = env::vars()
        .filter_map(|(k, _)| {
            k.strip_prefix("CARGO_FEATURE_")
                .map(|f| f.to_lowercase().replace('_', "-"))
        })
        .collect();
    names.sort();
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
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
    println!("cargo:rerun-if-env-changed=CODEMMET_GIT_BRANCH");
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
