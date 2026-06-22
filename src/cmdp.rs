use crate::utils;

// basically a single-line editor
pub struct CommandPalette {
    command: String,
    cursor: usize,
    // small enough for full snapshots
    undo_stack: Vec<(String, usize)>,
    redo_stack: Vec<(String, usize)>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            command: String::new(),
            cursor: 0usize,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn take_command(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.command)
    }

    pub fn get_command(&self) -> &str {
        &self.command
    }

    pub fn get_cursor(&self) -> usize {
        self.cursor
    }

    pub fn undo(&mut self) {
        if let Some((c, pos)) = self.undo_stack.pop() {
            self.redo_stack.push((std::mem::take(&mut self.command), self.cursor));
            self.command = c;
            self.cursor = pos;
        }
    }
    
    pub fn redo(&mut self) {
        if let Some((c, pos)) = self.redo_stack.pop() {
            self.undo_stack.push((std::mem::take(&mut self.command), self.cursor));
            self.command = c;
            self.cursor = pos;
        }
    }
    
    pub fn move_right(&mut self) {
        if self.cursor < self.command.chars().count() {
            self.cursor += 1;
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        self.redo_stack.clear();
        self.undo_stack.push((self.command.clone(), self.cursor));
        self.command.insert(utils::char_to_byte(self.cursor, &self.command), ch);
        self.cursor += 1;
    }

    pub fn delete_char(&mut self) {
        self.redo_stack.clear();
        self.undo_stack.push((self.command.clone(), self.cursor));
        if self.cursor > 0 {
            self.cursor -= 1;
            self.command.remove(utils::char_to_byte(self.cursor, &self.command));
        }
    }
    
    pub fn delete_char_front(&mut self) {
        self.redo_stack.clear();
        self.undo_stack.push((self.command.clone(), self.cursor));
        if self.cursor < self.command.chars().count() {
            self.command.remove(utils::char_to_byte(self.cursor, &self.command));
        }
    }
}