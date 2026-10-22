//! AuditLogRepository implementation for markdown backend.
//!
//! The audit log is append-only and can grow large, so instead of rewriting
//! a YAML document on every task edit it is kept as JSON Lines in
//! `audit_log.jsonl` under the data directory: one entry per line, appended
//! in place. A crash mid-append leaves at most one truncated trailing line,
//! which is skipped (with a warning) on the next load.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::domain::AuditLogEntry;
use crate::storage::{AuditLogRepository, StorageError, StorageResult};

use super::MarkdownBackend;

/// File name of the audit log inside the markdown data directory.
pub(crate) const AUDIT_LOG_FILE: &str = "audit_log.jsonl";

impl MarkdownBackend {
    fn audit_log_path(&self) -> PathBuf {
        self.base_path.join(AUDIT_LOG_FILE)
    }
}

impl AuditLogRepository for MarkdownBackend {
    fn append_audit_entry(&mut self, entry: &AuditLogEntry) -> StorageResult<()> {
        let path = self.audit_log_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| StorageError::io(parent, e))?;
        }
        let mut line =
            serde_json::to_string(entry).map_err(|e| StorageError::serialization(e.to_string()))?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| StorageError::io(&path, e))?;
        file.write_all(line.as_bytes())
            .map_err(|e| StorageError::io(&path, e))?;
        Ok(())
    }

    fn list_audit_entries(&self) -> StorageResult<Vec<AuditLogEntry>> {
        let path = self.audit_log_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&path).map_err(|e| StorageError::io(&path, e))?;

        let mut entries: Vec<AuditLogEntry> = Vec::new();
        for (line_no, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditLogEntry>(line) {
                Ok(entry) => {
                    // A retried append may have written the same ID twice; keep the latest.
                    if let Some(existing) = entries.iter_mut().find(|e| e.id == entry.id) {
                        *existing = entry;
                    } else {
                        entries.push(entry);
                    }
                }
                Err(e) => tracing::warn!(
                    "Skipping unreadable audit log line {} in {}: {e}",
                    line_no + 1,
                    path.display()
                ),
            }
        }
        Ok(entries)
    }
}
