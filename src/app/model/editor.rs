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
