//! System-level messages for application control.

/// System-level messages for application control.
///
/// These messages handle application lifecycle, persistence,
/// undo/redo, and export operations.
///
/// # Examples
///
/// ```
/// use taskflow::app::{Model, Message, SystemMessage, TaskMessage, update};
///
/// let mut model = Model::new();
///
/// // Create some tasks
/// update(&mut model, TaskMessage::Create("Task 1".to_string()).into());
/// update(&mut model, TaskMessage::Create("Task 2".to_string()).into());
///
/// // Undo the last action
/// update(&mut model, SystemMessage::Undo.into());
///
/// // Redo if needed
/// update(&mut model, SystemMessage::Redo.into());
/// ```
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// Quit the application
    Quit,
    /// Save current state to storage
    Save,
    /// Undo the last action
    Undo,
    /// Redo the last undone action
    Redo,
    /// Handle terminal resize
    Resize {
        /// New terminal width
        width: u16,
        /// New terminal height
        height: u16,
    },
    /// Periodic tick for time-based updates
    Tick,
    /// Export tasks to CSV format
    ExportCsv,
    /// Export tasks to ICS (iCalendar) format
    ExportIcs,
    /// Export task chains to DOT (Graphviz) format
    ExportChainsDot,
    /// Export task chains to Mermaid format
    ExportChainsMermaid,
    /// Export analytics report to Markdown format
    ExportReportMarkdown,
    /// Export analytics report to HTML format
    ExportReportHtml,
    /// Start import from CSV (opens file path input)
    StartImportCsv,
    /// Start import from ICS (opens file path input)
    StartImportIcs,
    /// Execute import after file path is entered
    ExecuteImport,
    /// Confirm pending import
    ConfirmImport,
    /// Cancel pending import
    CancelImport,
    /// Refresh storage to detect external file changes
    RefreshStorage,
}
