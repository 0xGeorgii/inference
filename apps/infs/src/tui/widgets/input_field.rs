#![warn(clippy::pedantic)]

//! Advanced input field widget with cursor support.
//!
//! This module provides an input field component that supports:
//! - Cursor positioning with Left/Right arrow keys
//! - Character insertion at cursor position
//! - Backspace and delete operations
//! - Command history navigation

/// State for an input field with cursor support.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct InputField {
    /// Current input text.
    content: String,
    /// Cursor position (byte offset in content).
    cursor: usize,
}

#[allow(dead_code)]
impl InputField {
    /// Creates a new empty input field.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current content.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the current cursor position.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the cursor position in display characters (for rendering).
    #[must_use]
    pub fn cursor_display_offset(&self) -> usize {
        self.content[..self.cursor].chars().count()
    }

    /// Returns true if the input is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Clears the input and resets cursor.
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    /// Sets the content and moves cursor to end.
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.cursor = self.content.len();
    }

    /// Inserts a character at the cursor position.
    pub fn insert(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Deletes the character before the cursor (Backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            // Find the start of the previous character
            let prev_char_start = self.content[..self.cursor]
                .char_indices()
                .last()
                .map_or(0, |(idx, _)| idx);
            self.content.remove(prev_char_start);
            self.cursor = prev_char_start;
        }
    }

    /// Deletes the character at the cursor position (Delete).
    #[allow(dead_code)]
    pub fn delete(&mut self) {
        if self.cursor < self.content.len() {
            self.content.remove(self.cursor);
        }
    }

    /// Moves cursor left by one character.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            // Find the start of the previous character
            self.cursor = self.content[..self.cursor]
                .char_indices()
                .last()
                .map_or(0, |(idx, _)| idx);
        }
    }

    /// Moves cursor right by one character.
    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            // Find the end of the current character
            if let Some((_, c)) = self.content[self.cursor..].char_indices().next() {
                self.cursor += c.len_utf8();
            }
        }
    }

    /// Moves cursor to the start of the input.
    #[allow(dead_code)]
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Moves cursor to the end of the input.
    #[allow(dead_code)]
    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    /// Returns the content as a trimmed lowercase string (for command matching).
    #[must_use]
    pub fn trimmed_lowercase(&self) -> String {
        self.content.trim().to_lowercase()
    }
}

/// Command history for navigating previous inputs.
#[derive(Debug, Clone, Default)]
pub struct CommandHistory {
    /// List of previous commands (oldest first).
    commands: Vec<String>,
    /// Current index in history (None means not navigating).
    index: Option<usize>,
    /// Maximum number of commands to store.
    max_size: usize,
    /// Temporary storage for current input when navigating.
    temp_input: String,
}

impl CommandHistory {
    /// Creates a new command history with default max size.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            index: None,
            max_size: 100,
            temp_input: String::new(),
        }
    }

    /// Creates a new command history with a specific max size.
    #[must_use]
    #[allow(dead_code)]
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            commands: Vec::new(),
            index: None,
            max_size,
            temp_input: String::new(),
        }
    }

    /// Adds a command to the history.
    pub fn push(&mut self, command: String) {
        // Don't add empty commands or duplicates of the last command
        if command.is_empty() {
            return;
        }
        if self.commands.last() == Some(&command) {
            return;
        }

        self.commands.push(command);

        // Trim to max size
        if self.commands.len() > self.max_size {
            self.commands.remove(0);
        }

        // Reset navigation
        self.index = None;
        self.temp_input.clear();
    }

    /// Navigates to the previous command (Up arrow).
    /// Returns the command to display, or None if no change.
    pub fn previous(&mut self, current_input: &str) -> Option<&str> {
        if self.commands.is_empty() {
            return None;
        }

        match self.index {
            None => {
                // Start navigating, save current input
                self.temp_input = current_input.to_string();
                self.index = Some(self.commands.len() - 1);
                self.commands.last().map(String::as_str)
            }
            Some(idx) if idx > 0 => {
                self.index = Some(idx - 1);
                self.commands.get(idx - 1).map(String::as_str)
            }
            Some(_) => {
                // Already at the oldest command
                None
            }
        }
    }

    /// Navigates to the next command (Down arrow).
    /// Returns the command to display, or None if no change.
    pub fn next(&mut self) -> Option<&str> {
        match self.index {
            Some(idx) if idx + 1 < self.commands.len() => {
                self.index = Some(idx + 1);
                self.commands.get(idx + 1).map(String::as_str)
            }
            Some(_) => {
                // Return to current input
                self.index = None;
                Some(self.temp_input.as_str())
            }
            None => None,
        }
    }

    /// Resets navigation state.
    pub fn reset_navigation(&mut self) {
        self.index = None;
        self.temp_input.clear();
    }

    /// Returns the number of commands in history.
    #[must_use]
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns true if history is empty.
    #[must_use]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_field_new_is_empty() {
        let field = InputField::new();
        assert!(field.is_empty());
        assert_eq!(field.cursor(), 0);
    }

    #[test]
    fn input_field_insert_updates_cursor() {
        let mut field = InputField::new();
        field.insert('a');
        assert_eq!(field.content(), "a");
        assert_eq!(field.cursor(), 1);

        field.insert('b');
        assert_eq!(field.content(), "ab");
        assert_eq!(field.cursor(), 2);
    }

    #[test]
    fn input_field_backspace_removes_char() {
        let mut field = InputField::new();
        field.set_content("ab");
        field.backspace();
        assert_eq!(field.content(), "a");
        assert_eq!(field.cursor(), 1);
    }

    #[test]
    fn input_field_backspace_at_start_does_nothing() {
        let mut field = InputField::new();
        field.backspace();
        assert!(field.is_empty());
        assert_eq!(field.cursor(), 0);
    }

    #[test]
    fn input_field_move_left_and_right() {
        let mut field = InputField::new();
        field.set_content("abc");
        assert_eq!(field.cursor(), 3);

        field.move_left();
        assert_eq!(field.cursor(), 2);

        field.move_left();
        assert_eq!(field.cursor(), 1);

        field.move_right();
        assert_eq!(field.cursor(), 2);
    }

    #[test]
    fn input_field_move_left_at_start_stays() {
        let mut field = InputField::new();
        field.set_content("ab");
        field.move_left();
        field.move_left();
        field.move_left();
        assert_eq!(field.cursor(), 0);
    }

    #[test]
    fn input_field_move_right_at_end_stays() {
        let mut field = InputField::new();
        field.set_content("ab");
        field.move_right();
        assert_eq!(field.cursor(), 2);
    }

    #[test]
    fn input_field_insert_at_middle() {
        let mut field = InputField::new();
        field.set_content("ac");
        field.move_left();
        field.insert('b');
        assert_eq!(field.content(), "abc");
        assert_eq!(field.cursor(), 2);
    }

    #[test]
    fn input_field_clear_resets() {
        let mut field = InputField::new();
        field.set_content("test");
        field.clear();
        assert!(field.is_empty());
        assert_eq!(field.cursor(), 0);
    }

    #[test]
    fn input_field_handles_unicode() {
        let mut field = InputField::new();
        field.insert('a');
        field.insert('\u{00e9}'); // e with accent
        field.insert('b');
        assert_eq!(field.content(), "a\u{00e9}b");

        field.move_left();
        field.move_left();
        assert_eq!(field.cursor_display_offset(), 1);
    }

    #[test]
    fn input_field_trimmed_lowercase() {
        let mut field = InputField::new();
        field.set_content("  HELLO  ");
        assert_eq!(field.trimmed_lowercase(), "hello");
    }

    #[test]
    fn command_history_push_and_previous() {
        let mut history = CommandHistory::new();
        history.push("cmd1".to_string());
        history.push("cmd2".to_string());

        let prev = history.previous("current");
        assert_eq!(prev, Some("cmd2"));

        let prev = history.previous("current");
        assert_eq!(prev, Some("cmd1"));

        // At the oldest command
        let prev = history.previous("current");
        assert_eq!(prev, None);
    }

    #[test]
    fn command_history_next_returns_to_current() {
        let mut history = CommandHistory::new();
        history.push("cmd1".to_string());
        history.push("cmd2".to_string());

        history.previous("current input");
        history.previous("current input");

        let next = history.next();
        assert_eq!(next, Some("cmd2"));

        let next = history.next();
        assert_eq!(next, Some("current input"));
    }

    #[test]
    fn command_history_empty_push_ignored() {
        let mut history = CommandHistory::new();
        history.push(String::new());
        assert!(history.is_empty());
    }

    #[test]
    fn command_history_duplicate_push_ignored() {
        let mut history = CommandHistory::new();
        history.push("cmd".to_string());
        history.push("cmd".to_string());
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn command_history_respects_max_size() {
        let mut history = CommandHistory::with_max_size(2);
        history.push("cmd1".to_string());
        history.push("cmd2".to_string());
        history.push("cmd3".to_string());
        assert_eq!(history.len(), 2);

        let prev = history.previous("");
        assert_eq!(prev, Some("cmd3"));

        let prev = history.previous("");
        assert_eq!(prev, Some("cmd2"));
    }

    #[test]
    fn command_history_reset_navigation() {
        let mut history = CommandHistory::new();
        history.push("cmd".to_string());
        history.previous("current");
        history.reset_navigation();

        // After reset, previous should start from the end again
        let prev = history.previous("new current");
        assert_eq!(prev, Some("cmd"));
    }
}
