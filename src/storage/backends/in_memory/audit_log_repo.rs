//! AuditLogRepository blanket implementation for in-memory backends.

use crate::domain::AuditLogEntry;
use crate::storage::{AuditLogRepository, StorageResult};

use super::InMemoryBackend;

impl<B: InMemoryBackend> AuditLogRepository for B {
    fn append_audit_entry(&mut self, entry: &AuditLogEntry) -> StorageResult<()> {
        let logs = &mut self.data_mut().audit_logs;
        if let Some(existing) = logs.iter_mut().find(|e| e.id == entry.id) {
            *existing = entry.clone();
        } else {
            logs.push(entry.clone());
        }
        self.mark_dirty();
        Ok(())
    }

    fn list_audit_entries(&self) -> StorageResult<Vec<AuditLogEntry>> {
        Ok(self.data().audit_logs.clone())
    }
}
