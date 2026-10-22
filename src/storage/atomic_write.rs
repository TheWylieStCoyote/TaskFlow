//! Crash-safe file writes.
//!
//! A plain `fs::write` truncates the target before writing, so a crash or
//! `kill -9` mid-write leaves an empty or half-written data file and the app
//! refuses to start on the next launch. Writing to a sibling temp file and then
//! renaming it over the target makes the replacement atomic on every platform
//! `std::fs::rename` supports.

use std::fs;
use std::path::Path;

use super::{StorageError, StorageResult};

/// Write `contents` to `path` atomically.
///
/// The data is first written to `<path>.tmp` in the same directory and then
/// renamed over `path`, so readers see either the old file or the complete new
/// one, never a partial write. The parent directory is created if missing.
///
/// # Errors
///
/// Returns [`StorageError::Io`] if the directory cannot be created, the temp
/// file cannot be written, or the rename fails.
pub fn atomic_write(path: &Path, contents: impl AsRef<[u8]>) -> StorageResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| StorageError::io(parent, e))?;
        }
    }

    let tmp_path = temp_path_for(path);
    fs::write(&tmp_path, contents).map_err(|e| StorageError::io(&tmp_path, e))?;

    if let Err(e) = fs::rename(&tmp_path, path) {
        // Best effort: don't leave a stray temp file behind.
        let _ = fs::remove_file(&tmp_path);
        return Err(StorageError::io(path, e));
    }
    Ok(())
}

/// `foo/bar.json` -> `foo/bar.json.tmp`
fn temp_path_for(path: &Path) -> std::path::PathBuf {
    let mut name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_new_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.json");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
        assert!(
            !temp_path_for(&path).exists(),
            "temp file should be renamed away"
        );
    }

    #[test]
    fn replaces_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.json");
        fs::write(&path, "old").unwrap();
        atomic_write(&path, "new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn creates_missing_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a").join("b").join("data.yaml");
        atomic_write(&path, "x").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn temp_path_is_a_sibling() {
        let p = Path::new("/some/dir/tasks.md");
        assert_eq!(temp_path_for(p), Path::new("/some/dir/tasks.md.tmp"));
    }
}
