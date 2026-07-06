//! Shared helpers for the `codemmet` binary's integration tests.
//!
//! Included by each integration-test file with `mod common;`. Living in a
//! subdirectory keeps Cargo from compiling it as its own test binary.

use std::path::PathBuf;

/// A unique, freshly created scratch directory for one test.
///
/// Keyed by the caller-supplied `name` plus the process id, so tests in the
/// same binary (which share a pid) stay isolated from one another.
pub fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("codemmet-test-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir should be creatable");
    dir
}
