//! StorageBackend implementation for YAML backend.

use crate::domain::{PomodoroConfig, PomodoroSession, PomodoroStats};
use crate::storage::{ExportData, StorageBackend, StorageResult};

use super::YamlBackend;

impl StorageBackend for YamlBackend {
    fn initialize(&mut self) -> StorageResult<()> {
        self.load()
    }

    fn flush(&mut self) -> StorageResult<()> {
        self.save()
    }

    fn export_all(&self) -> StorageResult<ExportData> {
        Ok(self.data.clone())
    }

    fn import_all(&mut self, data: &ExportData) -> StorageResult<()> {
        self.data = data.clone();
        self.mark_dirty();
        self.save()
    }

    fn backend_type(&self) -> &'static str {
        "yaml"
    }

    fn set_pomodoro_session(&mut self, session: Option<&PomodoroSession>) -> StorageResult<()> {
        self.data.pomodoro_session = session.cloned();
        self.mark_dirty();
        Ok(())
    }

    fn set_pomodoro_config(&mut self, config: &PomodoroConfig) -> StorageResult<()> {
        self.data.pomodoro_config = Some(config.clone());
        self.mark_dirty();
        Ok(())
    }

    fn set_pomodoro_stats(&mut self, stats: &PomodoroStats) -> StorageResult<()> {
        self.data.pomodoro_stats = Some(stats.clone());
        self.mark_dirty();
        Ok(())
    }
}
