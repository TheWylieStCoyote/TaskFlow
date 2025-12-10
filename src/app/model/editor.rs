//! Multi-line text editor trait and implementations.
//!
//! This module provides a trait for multi-line text editing with common
//! operations like cursor movement, character insertion, and line manipulation.

/// Trait for multi-line text editors.
///
/// Provides default implementations for common editing operations when the
/// implementing type provides access to the buffer and cursor state.
pub trait MultilineEditor {
    /// Get the text buffer (lines of text)
    fn buffer(&self) -> &[String];

    /// Get mutable access to the text buffer
    fn buffer_mut(&mut self) -> &mut Vec<String>;

    /// Get the current cursor line position
    fn cursor_line(&self) -> usize;

    /// Get the current cursor column position
    fn cursor_col(&self) -> usize;

    /// Set the cursor position
    fn set_cursor(&mut self, line: usize, col: usize);

    /// Ensure buffer has at least one line
    fn ensure_buffer_not_empty(&mut self) {
        if self.buffer().is_empty() {
            self.buffer_mut().push(String::new());
        }
    }

    /// Insert a character at the current cursor position
    fn insert_char(&mut self, c: char) {
        self.ensure_buffer_not_empty();

        let line_idx = self
            .cursor_line()
            .min(self.buffer().len().saturating_sub(1));
        let col = self.cursor_col().min(self.buffer()[line_idx].len());

        self.buffer_mut()[line_idx].insert(col, c);
        self.set_cursor(line_idx, col + 1);
    }

    /// Delete the character before the cursor (backspace)
    fn backspace(&mut self) {
        let line_idx = self.cursor_line();
        let buffer_len = self.buffer().len();

        if line_idx < buffer_len {
            if self.cursor_col() > 0 {
                // Delete character before cursor
                let col = self.cursor_col().min(self.buffer()[line_idx].len());
                if col > 0 {
                    self.buffer_mut()[line_idx].remove(col - 1);
                    self.set_cursor(line_idx, col - 1);
                }
            } else if line_idx > 0 {
                // At beginning of line - join with previous line
                let current_line = self.buffer_mut().remove(line_idx);
                let prev_line_len = self.buffer()[line_idx - 1].len();
                self.buffer_mut()[line_idx - 1].push_str(&current_line);
                self.set_cursor(line_idx - 1, prev_line_len);
            }
        }
    }

    /// Delete the character at the cursor position
    fn delete_char(&mut self) {
        let line_idx = self.cursor_line();
        let buffer_len = self.buffer().len();

        if line_idx < buffer_len {
            let col = self.cursor_col();
            let line_len = self.buffer()[line_idx].len();

            if col < line_len {
                // Delete character at cursor
                self.buffer_mut()[line_idx].remove(col);
            } else if line_idx + 1 < buffer_len {
                // At end of line - join with next line
                let next_line = self.buffer_mut().remove(line_idx + 1);
                self.buffer_mut()[line_idx].push_str(&next_line);
            }
        }
    }

    /// Move cursor left (with line wrapping)
    fn cursor_left(&mut self) {
        if self.cursor_col() > 0 {
            self.set_cursor(self.cursor_line(), self.cursor_col() - 1);
        } else if self.cursor_line() > 0 {
            // Move to end of previous line
            let prev_line = self.cursor_line() - 1;
            let prev_line_len = self.buffer()[prev_line].len();
            self.set_cursor(prev_line, prev_line_len);
        }
    }

    /// Move cursor right (with line wrapping)
    fn cursor_right(&mut self) {
        let line_idx = self.cursor_line();
        let buffer_len = self.buffer().len();

        if line_idx < buffer_len {
            let line_len = self.buffer()[line_idx].len();
            if self.cursor_col() < line_len {
                self.set_cursor(line_idx, self.cursor_col() + 1);
            } else if line_idx + 1 < buffer_len {
                // Move to beginning of next line
                self.set_cursor(line_idx + 1, 0);
            }
        }
    }

    /// Move cursor up (with column clamping)
    fn cursor_up(&mut self) {
        if self.cursor_line() > 0 {
            let new_line = self.cursor_line() - 1;
            let new_line_len = self.buffer()[new_line].len();
            let new_col = self.cursor_col().min(new_line_len);
            self.set_cursor(new_line, new_col);
        }
    }

    /// Move cursor down (with column clamping)
    fn cursor_down(&mut self) {
        let buffer_len = self.buffer().len();
        if self.cursor_line() + 1 < buffer_len {
            let new_line = self.cursor_line() + 1;
            let new_line_len = self.buffer()[new_line].len();
            let new_col = self.cursor_col().min(new_line_len);
            self.set_cursor(new_line, new_col);
        }
    }

    /// Move cursor to start of line
    fn cursor_home(&mut self) {
        self.set_cursor(self.cursor_line(), 0);
    }

    /// Move cursor to end of line
    fn cursor_end(&mut self) {
        let line_idx = self.cursor_line();
        if line_idx < self.buffer().len() {
            let line_len = self.buffer()[line_idx].len();
            self.set_cursor(line_idx, line_len);
        }
    }

    /// Insert a newline at the cursor position (split line)
    fn newline(&mut self) {
        let line_idx = self.cursor_line();
        if line_idx < self.buffer().len() {
            let col = self.cursor_col().min(self.buffer()[line_idx].len());
            let remainder = self.buffer_mut()[line_idx].split_off(col);
            self.buffer_mut().insert(line_idx + 1, remainder);
            self.set_cursor(line_idx + 1, 0);
        }
    }

    /// Clear the buffer and reset cursor
    fn clear(&mut self) {
        self.buffer_mut().clear();
        self.buffer_mut().push(String::new());
        self.set_cursor(0, 0);
    }

    /// Get the content as a single string with newlines
    fn content(&self) -> String {
        self.buffer().join("\n")
    }

    /// Set content from a string, splitting into lines
    fn set_content(&mut self, content: &str) {
        self.buffer_mut().clear();
        if content.is_empty() {
            self.buffer_mut().push(String::new());
        } else {
            self.buffer_mut().extend(content.lines().map(String::from));
        }
        if self.buffer().is_empty() {
            self.buffer_mut().push(String::new());
        }
        self.set_cursor(0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test editor implementation
    struct TestEditor {
        buffer: Vec<String>,
        cursor_line: usize,
        cursor_col: usize,
    }

    impl TestEditor {
        fn new() -> Self {
            Self {
                buffer: vec![String::new()],
                cursor_line: 0,
                cursor_col: 0,
            }
        }
    }

    impl MultilineEditor for TestEditor {
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

    #[test]
    fn test_insert_char() {
        let mut editor = TestEditor::new();
        editor.insert_char('a');
        editor.insert_char('b');
        editor.insert_char('c');

        assert_eq!(editor.content(), "abc");
        assert_eq!(editor.cursor_col(), 3);
    }

    #[test]
    fn test_backspace() {
        let mut editor = TestEditor::new();
        editor.set_content("abc");
        editor.set_cursor(0, 3);

        editor.backspace();
        assert_eq!(editor.content(), "ab");
        assert_eq!(editor.cursor_col(), 2);
    }

    #[test]
    fn test_backspace_at_line_start_joins_lines() {
        let mut editor = TestEditor::new();
        editor.set_content("line1\nline2");
        editor.set_cursor(1, 0);

        editor.backspace();
        assert_eq!(editor.content(), "line1line2");
        assert_eq!(editor.cursor_line(), 0);
        assert_eq!(editor.cursor_col(), 5);
    }

    #[test]
    fn test_delete_char() {
        let mut editor = TestEditor::new();
        editor.set_content("abc");
        editor.set_cursor(0, 1);

        editor.delete_char();
        assert_eq!(editor.content(), "ac");
    }

    #[test]
    fn test_delete_char_at_line_end_joins_lines() {
        let mut editor = TestEditor::new();
        editor.set_content("line1\nline2");
        editor.set_cursor(0, 5);

        editor.delete_char();
        assert_eq!(editor.content(), "line1line2");
    }

    #[test]
    fn test_cursor_left() {
        let mut editor = TestEditor::new();
        editor.set_content("abc");
        editor.set_cursor(0, 2);

        editor.cursor_left();
        assert_eq!(editor.cursor_col(), 1);
    }

    #[test]
    fn test_cursor_left_wraps_to_previous_line() {
        let mut editor = TestEditor::new();
        editor.set_content("line1\nline2");
        editor.set_cursor(1, 0);

        editor.cursor_left();
        assert_eq!(editor.cursor_line(), 0);
        assert_eq!(editor.cursor_col(), 5);
    }

    #[test]
    fn test_cursor_right() {
        let mut editor = TestEditor::new();
        editor.set_content("abc");
        editor.set_cursor(0, 1);

        editor.cursor_right();
        assert_eq!(editor.cursor_col(), 2);
    }

    #[test]
    fn test_cursor_right_wraps_to_next_line() {
        let mut editor = TestEditor::new();
        editor.set_content("line1\nline2");
        editor.set_cursor(0, 5);

        editor.cursor_right();
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_col(), 0);
    }

    #[test]
    fn test_cursor_up() {
        let mut editor = TestEditor::new();
        editor.set_content("line1\nline2");
        editor.set_cursor(1, 2);

        editor.cursor_up();
        assert_eq!(editor.cursor_line(), 0);
        assert_eq!(editor.cursor_col(), 2);
    }

    #[test]
    fn test_cursor_down() {
        let mut editor = TestEditor::new();
        editor.set_content("line1\nline2");
        editor.set_cursor(0, 2);

        editor.cursor_down();
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_col(), 2);
    }

    #[test]
    fn test_cursor_home() {
        let mut editor = TestEditor::new();
        editor.set_content("abc");
        editor.set_cursor(0, 3);

        editor.cursor_home();
        assert_eq!(editor.cursor_col(), 0);
    }

    #[test]
    fn test_cursor_end() {
        let mut editor = TestEditor::new();
        editor.set_content("abc");
        editor.set_cursor(0, 0);

        editor.cursor_end();
        assert_eq!(editor.cursor_col(), 3);
    }

    #[test]
    fn test_newline() {
        let mut editor = TestEditor::new();
        editor.set_content("abcdef");
        editor.set_cursor(0, 3);

        editor.newline();
        assert_eq!(editor.buffer.len(), 2);
        assert_eq!(editor.buffer[0], "abc");
        assert_eq!(editor.buffer[1], "def");
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_col(), 0);
    }

    #[test]
    fn test_clear() {
        let mut editor = TestEditor::new();
        editor.set_content("line1\nline2\nline3");
        editor.set_cursor(1, 2);

        editor.clear();
        assert_eq!(editor.buffer.len(), 1);
        assert_eq!(editor.buffer[0], "");
        assert_eq!(editor.cursor_line(), 0);
        assert_eq!(editor.cursor_col(), 0);
    }

    #[test]
    fn test_content() {
        let mut editor = TestEditor::new();
        editor.buffer = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];

        assert_eq!(editor.content(), "line1\nline2\nline3");
    }

    #[test]
    fn test_set_content() {
        let mut editor = TestEditor::new();
        editor.set_content("line1\nline2\nline3");

        assert_eq!(editor.buffer.len(), 3);
        assert_eq!(editor.buffer[0], "line1");
        assert_eq!(editor.buffer[1], "line2");
        assert_eq!(editor.buffer[2], "line3");
    }

    #[test]
    fn test_set_content_empty() {
        let mut editor = TestEditor::new();
        editor.set_content("");

        assert_eq!(editor.buffer.len(), 1);
        assert_eq!(editor.buffer[0], "");
    }

    #[test]
    fn test_ensure_buffer_not_empty() {
        let mut editor = TestEditor::new();
        editor.buffer.clear();

        editor.ensure_buffer_not_empty();
        assert_eq!(editor.buffer.len(), 1);
    }

    #[test]
    fn test_cursor_up_clamps_column() {
        let mut editor = TestEditor::new();
        editor.set_content("ab\nline two");
        editor.set_cursor(1, 8);

        editor.cursor_up();
        assert_eq!(editor.cursor_line(), 0);
        assert_eq!(editor.cursor_col(), 2); // Clamped to shorter line
    }

    #[test]
    fn test_cursor_down_clamps_column() {
        let mut editor = TestEditor::new();
        editor.set_content("long line\nab");
        editor.set_cursor(0, 9);

        editor.cursor_down();
        assert_eq!(editor.cursor_line(), 1);
        assert_eq!(editor.cursor_col(), 2); // Clamped to shorter line
    }
}
