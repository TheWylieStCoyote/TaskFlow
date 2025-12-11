//! Editor state types for multi-line text editing.

/// State for multi-line description editor.
#[derive(Debug, Clone)]
pub struct DescriptionEditorState {
    /// Whether description editor is visible
    pub visible: bool,
    /// Text buffer for editing description (multi-line)
    pub buffer: Vec<String>,
    /// Cursor line position in description buffer
    pub cursor_line: usize,
    /// Cursor column position in description buffer
    pub cursor_col: usize,
}

impl Default for DescriptionEditorState {
    fn default() -> Self {
        Self {
            visible: false,
            buffer: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
        }
    }
}

impl super::super::MultilineEditor for DescriptionEditorState {
    fn buffer(&self) -> &[String] {
        &self.buffer
    }

    fn buffer_mut(&mut self) -> &mut Vec<String> {
        &mut self.buffer
    }

    fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    fn set_cursor(&mut self, line: usize, col: usize) {
        self.cursor_line = line;
        self.cursor_col = col;
    }
}

/// State for work log editor modal.
#[derive(Debug, Clone)]
pub struct WorkLogEditorState {
    /// Whether work log editor is visible
    pub visible: bool,
    /// Selected work log entry index
    pub selected: usize,
    /// Current mode in work log editor
    pub mode: crate::ui::WorkLogMode,
    /// Text buffer for editing work log entries (multi-line)
    pub buffer: Vec<String>,
    /// Cursor line position in work log buffer
    pub cursor_line: usize,
    /// Cursor column position in work log buffer
    pub cursor_col: usize,
    /// Search query for filtering work log entries
    pub search_query: String,
}

impl Default for WorkLogEditorState {
    fn default() -> Self {
        Self {
            visible: false,
            selected: 0,
            mode: crate::ui::WorkLogMode::default(),
            buffer: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            search_query: String::new(),
        }
    }
}

impl super::super::MultilineEditor for WorkLogEditorState {
    fn buffer(&self) -> &[String] {
        &self.buffer
    }

    fn buffer_mut(&mut self) -> &mut Vec<String> {
        &mut self.buffer
    }

    fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    fn set_cursor(&mut self, line: usize, col: usize) {
        self.cursor_line = line;
        self.cursor_col = col;
    }
}

/// State for time log editor modal.
#[derive(Debug, Clone, Default)]
pub struct TimeLogEditorState {
    /// Whether time log editor is visible
    pub visible: bool,
    /// Selected time entry index in log
    pub selected: usize,
    /// Current mode in time log editor
    pub mode: crate::ui::TimeLogMode,
    /// Text buffer for editing time entries
    pub buffer: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::MultilineEditor;

    #[test]
    fn test_description_editor_state_default() {
        let state = DescriptionEditorState::default();
        assert!(!state.visible);
        assert_eq!(state.buffer, vec![String::new()]);
        assert_eq!(state.cursor_line, 0);
        assert_eq!(state.cursor_col, 0);
    }

    #[test]
    fn test_description_editor_implements_multiline_editor() {
        let mut state = DescriptionEditorState::default();
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');

        assert_eq!(state.content(), "abc");
    }

    #[test]
    fn test_work_log_editor_state_default() {
        let state = WorkLogEditorState::default();
        assert!(!state.visible);
        assert_eq!(state.selected, 0);
        assert_eq!(state.buffer, vec![String::new()]);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_work_log_editor_implements_multiline_editor() {
        let mut state = WorkLogEditorState::default();
        state.set_content("line 1\nline 2");

        assert_eq!(state.buffer.len(), 2);
        assert_eq!(state.content(), "line 1\nline 2");
    }
}
