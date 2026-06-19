pub struct Editor {
    buffer_lines: Vec<String>,
    cursor_pos: (usize, usize)
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer_lines: vec![String::new()],
            cursor_pos: (0, 0),
        }
    }

    pub fn from_buffer<S: Into<String>>(buffer: S) -> Self {
        let buf_str: String = buffer.into();
        if buf_str.is_empty() {
            Self {
                buffer_lines: vec![String::new()],
                cursor_pos: (0, 0),
            }
        } else {
            Self {
                buffer_lines: buf_str.lines()
                    .map(|x| x.to_string())
                    .collect(),
                cursor_pos: (0, 0),
            }
        }
    }
    
    pub fn get_buffer_lines(&self) -> &[String] {
        &self.buffer_lines
    }

    pub fn get_pos(&self) -> (usize, usize) {
        self.cursor_pos
    }
    
    pub fn move_right(&mut self) {
        let line = &self.buffer_lines[self.cursor_pos.1 as usize];
        let (cx, cy) = self.cursor_pos;
        let line_len = line.chars().count();
        
        if cx < line_len {
            self.cursor_pos.0 = cx + 1;
        } else if cy as usize + 1 < self.buffer_lines.len() {
            self.cursor_pos.0 = 0;
            self.cursor_pos.1 = cy + 1;
        }
    }
    
    pub fn move_left(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if cx > 0 {
            self.cursor_pos.0 = cx.saturating_sub(1);
        } else if cy > 0 {
            self.cursor_pos.1 = cy.saturating_sub(1);
            let prev_line = &self.buffer_lines[self.cursor_pos.1 as usize];
            self.cursor_pos.0 = prev_line.chars().count();
        }
    }

    pub fn move_up(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if cy > 0 {
            self.cursor_pos.1 = cy - 1;
            let next_line_len = self.buffer_lines[cy as usize - 1].chars().count();
            self.cursor_pos.0 = next_line_len.min(cx);
        }
    }
    
    pub fn move_down(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if let Some(next_line) = self.buffer_lines.get(cy as usize + 1) {
            self.cursor_pos.1 = cy + 1;
            let next_line_len = next_line.chars().count();
            self.cursor_pos.0 = next_line_len.min(cx);
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let (cx, cy) = self.cursor_pos;
        if ch == '\n' {
            let line = &mut self.buffer_lines[self.cursor_pos.1 as usize];
            let indent = line.chars()
                .take_while(|c| c.is_whitespace() && *c != '\n')
                .collect::<String>();
            let split = line.split_off(cx as usize);
            let nl = format!("{}{}", indent, split);
            self.cursor_pos.1 = cy + 1;
            self.buffer_lines.insert(self.cursor_pos.1 as usize, nl);
            self.cursor_pos.0 = indent.chars().count();
        } else {
            self.buffer_lines[cy as usize].insert(cx as usize, ch);
            self.cursor_pos.0 = cx + 1;
        }
    }
    
    pub fn insert_str(&mut self, s: &str) {
        let (cx, cy) = self.cursor_pos;
        self.buffer_lines[cy as usize].insert_str(cx as usize, s);
        self.cursor_pos.0 = cx + s.len();
    }
    
    pub fn delete_char(&mut self) {
        let (cx, cy) = self.cursor_pos;
        
        if cx > 0 {
            let line = &mut self.buffer_lines[self.cursor_pos.1 as usize];
            if line[..cx].chars().all(|c| c == ' ') {
                self.cursor_pos.0 = 0;
                line.drain(..cx);
            } else {
                self.cursor_pos.0 = cx - 1;
                line.remove(self.cursor_pos.0);
            }
        } else if cy > 0 {
            self.cursor_pos.1 = cy - 1;
            self.cursor_pos.0 = self.buffer_lines[cy as usize - 1]
                .chars().count();
            if let Some(l) = self.buffer_lines
                .get_mut(cy as usize)
                .map(|line| std::mem::take(line))
            {
                self.buffer_lines[cy as usize - 1].push_str(&l);
                self.buffer_lines.remove(cy as usize);
            }
        }
    }
}