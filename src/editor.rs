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
        let line = &self.buffer_lines[self.cursor_pos.1];
        let (cx, cy) = self.cursor_pos;
        let line_len = line.chars().count();
        
        if cx < line_len {
            self.cursor_pos.0 = cx + 1;
        } else if cy + 1 < self.buffer_lines.len() {
            self.cursor_pos.0 = 0;
            self.cursor_pos.1 = cy + 1;
        }
    }
    
    pub fn move_left(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if cx > 0 {
            self.cursor_pos.0 = cx - 1;
        } else if cy > 0 {
            self.cursor_pos.1 = cy - 1;
            let prev_line = &self.buffer_lines[self.cursor_pos.1];
            self.cursor_pos.0 = prev_line.chars().count();
        }
    }

    pub fn move_up(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if cy > 0 {
            self.cursor_pos.1 = cy - 1;
            let next_line_len = self.buffer_lines[cy - 1].chars().count();
            self.cursor_pos.0 = next_line_len.min(cx);
        } else {
            self.cursor_pos.0 = 0;
        }
    }
    
    pub fn move_down(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if let Some(next_line) = self.buffer_lines.get(cy + 1) {
            self.cursor_pos.1 = cy + 1;
            let next_line_len = next_line.chars().count();
            self.cursor_pos.0 = next_line_len.min(cx);
        } else {
            self.cursor_pos.0 = self.buffer_lines[cy].chars().count();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let (cx, cy) = self.cursor_pos;
        let line = &mut self.buffer_lines[cy];
        let cxb = Self::char_to_byte(cx, line);
        if ch == '\n' {
            let indent = line.chars()
                .take_while(|c| c.is_whitespace() && *c != '\n')
                .collect::<String>();
            let split = line.split_off(cxb);
            let nl = format!("{}{}", indent, split);
            self.cursor_pos.1 = cy + 1;
            self.buffer_lines.insert(self.cursor_pos.1, nl);
            self.cursor_pos.0 = indent.chars().count();
        } else {
            line.insert(cxb, ch);
            self.cursor_pos.0 = cx + 1;
        }
    }
    
    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.insert_char(ch);
        }
    }
    
    pub fn delete_char(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if cx > 0 {
            let line = &mut self.buffer_lines[self.cursor_pos.1];
            let cxb = Self::char_to_byte(cx, line);
            if line.chars().take(cx).all(|c| c == ' ') {
                self.cursor_pos.0 = 0;
                line.drain(..cxb);
            } else {
                self.cursor_pos.0 = cx - 1;
                line.remove(Self::char_to_byte(self.cursor_pos.0, line));
            }
        } else if cy > 0 {
            self.cursor_pos.1 = cy - 1;
            self.cursor_pos.0 = self.buffer_lines[cy - 1].chars().count();
            if let Some(l) = self.buffer_lines
                .get_mut(cy)
                .map(|line| std::mem::take(line))
            {
                self.buffer_lines[cy - 1].push_str(&l);
                self.buffer_lines.remove(cy);
            }
        }
    }
    
    pub fn delete_char_front(&mut self) {
        let (cx, cy) = self.cursor_pos;

        let line = &mut self.buffer_lines[self.cursor_pos.1];
        let cxb = Self::char_to_byte(cx, line);
        if cx >= line.chars().count() {
            if let Some(l) = self.buffer_lines
                .get_mut(cy + 1)
                .map(|line| std::mem::take(line))
            {
                self.buffer_lines[cy].push_str(&l);
                self.buffer_lines.remove(cy + 1);
            }
        } else if cx > 0 && line.chars().skip(cx - 1).all(|c| c == ' ') {
            line.drain((
                cxb
                + line.chars().nth(cx + 1).unwrap().len_utf8()
            )..);
        } else {
            line.remove(cxb);
        }
    }

    pub fn home(&mut self) {
        let (cx, _) = self.cursor_pos;
        if cx > 0 {
            self.cursor_pos.0 = 0;
        } else {
            self.cursor_pos.0 = 0;
            self.cursor_pos.1 = 0;
        }
    }
    
    pub fn end(&mut self) {
        let (cx, cy) = self.cursor_pos;
        let current_line = self.buffer_lines[cy].chars().count();
        if cx < current_line {
            self.cursor_pos.0 = current_line;
        } else {
            self.cursor_pos.1 = self.buffer_lines.len() - 1;
            self.cursor_pos.0 = self.buffer_lines[self.cursor_pos.1].chars().count();
        }
    }

    // this is kinda slow
    fn char_to_byte(pos: usize, s: &str) -> usize {
        s.chars()
            .take(pos)
            .map(|ch| ch.len_utf8())
            .sum()
    }
}