use crate::utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineId(usize);

impl Default for LineId {
    fn default() -> Self {
        Self(0) // not used anyway
    }
}

enum ExtraAction {
    Restore(LineId, String),
    Delete(LineId),
    None,
}

struct Action {
    id: LineId,
    new: String,
    extra: ExtraAction,
}

// todo: improve byte-to-char index conversions
pub struct Editor {
    buffer_lines: Vec<(LineId, String)>,
    cursor_pos: (usize, usize),
    scroll_offset: usize,
    next_line_id: usize,
    undo_stack: Vec<(Action, (usize, usize))>,
    redo_stack: Vec<(Action, (usize, usize))>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer_lines: vec![(LineId(0), String::new())],
            cursor_pos: (0, 0),
            scroll_offset: 0,
            next_line_id: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    fn create_line_id(&mut self) -> LineId {
        let id = LineId(self.next_line_id);
        self.next_line_id += 1;
        id
    }

    pub fn from_buffer<S: Into<String>>(buffer: S) -> Self {
        let buf_str: String = buffer.into();
        if buf_str.is_empty() {
            Self::new()
        } else {
            let mut buffer_lines = Vec::new();
            let mut next_line_id = 0;
            for (idx, line) in buf_str.lines().enumerate() {
                buffer_lines.push((LineId(idx), line.to_string()));
                next_line_id = idx + 1;
            }
            Self {
                buffer_lines, next_line_id,
                ..Self::new()
            }
        }
    }

    pub fn undo(&mut self) {
        if let Some((a, pos)) = self.undo_stack.pop() {
            let mut searched_main_idx = None;
            let extra = match a.extra {
                ExtraAction::Restore(id, s) => {
                    let idx = self.buffer_lines.iter()
                        .position(|l| l.0 == a.id)
                        .unwrap();
                    self.buffer_lines.insert(idx + 1, (id, s));
                    searched_main_idx = Some(idx);
                    ExtraAction::Delete(id)
                },
                ExtraAction::Delete(id) => {
                    let idx = self.buffer_lines.iter_mut()
                        .position(|l| l.0 == id).unwrap();
                    let contents = self.buffer_lines.remove(idx);
                    ExtraAction::Restore(id, contents.1)
                },
                ExtraAction::None => ExtraAction::None,
            };
            if let Some(idx) = searched_main_idx {
                let (_, l) = &mut self.buffer_lines[idx];
                self.redo_stack.push((
                    Action {
                        id: a.id,
                        new: std::mem::take(l),
                        extra
                    },
                    self.cursor_pos
                ));
                *l = a.new;
            } else if let Some((_, l)) = self.buffer_lines.iter_mut().find(|l| l.0 == a.id) {
                self.redo_stack.push((
                    Action {
                        id: a.id,
                        new: std::mem::take(l),
                        extra
                    },
                    self.cursor_pos
                ));
                *l = a.new;
            }
            self.cursor_pos = pos;
        }
    }
    
    pub fn redo(&mut self) {
        if let Some((a, pos)) = self.redo_stack.pop() {
            // avoid recomputation
            let mut searched_main_idx = None;
            let extra = match a.extra {
                ExtraAction::Restore(id, s) => {
                    let idx = self.buffer_lines.iter()
                        .position(|l| l.0 == a.id)
                        .unwrap();
                    self.buffer_lines.insert(idx + 1, (id, s));
                    searched_main_idx = Some(idx);
                    ExtraAction::Delete(id)
                },
                ExtraAction::Delete(id) => {
                    let idx = self.buffer_lines.iter_mut()
                        .position(|l| l.0 == id).unwrap();
                    let contents = self.buffer_lines.remove(idx);
                    ExtraAction::Restore(id, contents.1)
                },
                ExtraAction::None => ExtraAction::None,
            };
            if let Some(idx) = searched_main_idx {
                let (_, l) = &mut self.buffer_lines[idx];
                self.undo_stack.push((
                    Action {
                        id: a.id,
                        new: std::mem::take(l),
                        extra
                    },
                    self.cursor_pos
                ));
                *l = a.new;
            } else if let Some((_, l)) = self.buffer_lines.iter_mut().find(|l| l.0 == a.id) {
                self.undo_stack.push((
                    Action {
                        id: a.id,
                        new: std::mem::take(l),
                        extra
                    },
                    self.cursor_pos
                ));
                *l = a.new;
            }
            self.cursor_pos = pos;
        }
    }
    
    pub fn get_scroll_amount(&self) -> usize {
        self.scroll_offset
    }
    
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }
    
    pub fn scroll_down(&mut self, n: usize) {
        let max_scroll = self.buffer_lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + n).min(max_scroll);
    }
    
    pub fn get_visible_buffer_lines(&self, term_rows: u16) -> &[(LineId, String)] {
        if self.scroll_offset >= self.buffer_lines.len() {
            &[]
        } else {
            let view_end = self.scroll_offset + term_rows as usize;
            &self.buffer_lines[self.scroll_offset..view_end.min(self.buffer_lines.len())]
        }
    }
    
    pub fn get_full_buffer(&self) -> String {
        self.buffer_lines
            .iter()
            .map(|(_, l)| l.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn get_pos(&self) -> (usize, usize) {
        self.cursor_pos
    }
    
    pub fn move_right(&mut self) {
        let (cx, cy) = self.cursor_pos;
        let line = &self.buffer_lines[cy];
        let line_len = line.1.chars().count();
        
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
            self.cursor_pos.0 = self.buffer_lines[self.cursor_pos.1].1.chars().count();
        }
    }

    pub fn move_up(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if cy > 0 {
            self.cursor_pos.1 = cy - 1;
            let next_line_len = self.buffer_lines[cy - 1].1.chars().count();
            self.cursor_pos.0 = next_line_len.min(cx);
        } else {
            self.cursor_pos.0 = 0;
        }
    }
    
    pub fn move_down(&mut self) {
        let (cx, cy) = self.cursor_pos;
        if let Some(next_line) = self.buffer_lines.get(cy + 1) {
            self.cursor_pos.1 = cy + 1;
            let next_line_len = next_line.1.chars().count();
            self.cursor_pos.0 = next_line_len.min(cx);
        } else {
            self.cursor_pos.0 = self.buffer_lines[cy].1.chars().count();
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let (cx, cy) = self.cursor_pos;
        let (line_id, line) = &self.buffer_lines[cy];
        self.redo_stack.clear();
        let cxb = utils::char_to_byte(cx, line);
        if ch == '\n' {
            let indent = line.chars()
                .take_while(|c| c.is_whitespace() && *c != '\n')
                .collect::<String>();
            self.cursor_pos.1 = cy + 1;
            let (line_id, new) = (*line_id, line.clone());
            let id = self.create_line_id();
            self.undo_stack.push((
                Action {
                    id: line_id,
                    new,
                    extra: ExtraAction::Delete(id),
                },
                (cx, cy)
            ));
            let split = self.buffer_lines[cy].1.split_off(cxb);
            let nl = format!("{}{}", indent, split);
            self.buffer_lines.insert(self.cursor_pos.1, (id, nl));
            self.cursor_pos.0 = indent.chars().count();
        } else {
            self.undo_stack.push((
                Action {
                    id: *line_id,
                    new: line.clone(),
                    extra: ExtraAction::None,
                },
                (cx, cy)
            ));
            self.buffer_lines[cy].1.insert(cxb, ch);
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
            let (line_id, line) = &mut self.buffer_lines[self.cursor_pos.1];
            self.redo_stack.clear();
            self.undo_stack.push((
                Action {
                    id: *line_id,
                    new: line.clone(),
                    extra: ExtraAction::None,
                },
                (cx, cy)
            ));
            let cxb = utils::char_to_byte(cx, line);
            if line.chars().take(cx).all(|c| c == ' ') {
                self.cursor_pos.0 = 0;
                line.drain(..cxb);
            } else {
                self.cursor_pos.0 = cx - 1;
                line.remove(utils::char_to_byte(self.cursor_pos.0, line));
            }
        } else if cy > 0 {
            self.cursor_pos.1 = cy - 1;
            self.cursor_pos.0 = self.buffer_lines[cy - 1].1.chars().count();
            if let Some((id, l)) = self.buffer_lines
                .get_mut(cy)
                .map(std::mem::take)
            {
                self.redo_stack.clear();
                let prev = &mut self.buffer_lines[cy - 1];
                self.undo_stack.push((
                    Action {
                        id: prev.0,
                        new: prev.1.clone(),
                        extra: {
                            prev.1.push_str(&l);
                            ExtraAction::Restore(id, l)
                        },
                    },
                    (cx, cy)
                ));
                self.buffer_lines.remove(cy);
            }
        }
    }
    
    pub fn delete_char_front(&mut self) {
        let (cx, cy) = self.cursor_pos;

        let (line_id, line) = &self.buffer_lines[cy];
        let cxb = utils::char_to_byte(cx, line);
        if cx >= line.chars().count() {
            if let Some((id, l)) = self.buffer_lines
                .get_mut(cy + 1)
                .map(std::mem::take)
            {
                self.redo_stack.clear();
                let prev = &mut self.buffer_lines[cy];
                self.undo_stack.push((
                    Action {
                        id: prev.0,
                        new: prev.1.clone(),
                        extra: {
                            prev.1.push_str(&l);
                            ExtraAction::Restore(id, l)
                        },
                    },
                    (cx, cy)
                ));
                self.buffer_lines.remove(cy + 1);
            }
        } else if cx > 0 && line.chars().skip(cx - 1).all(|c| c == ' ') {
            let cxb1 = utils::char_to_byte(cx + 1, line);
            self.redo_stack.clear();
            self.undo_stack.push((
                Action {
                    id: *line_id,
                    new: line.clone(),
                    extra: ExtraAction::None,
                },
                (cx, cy)
            ));
            self.buffer_lines[cy].1.drain(cxb1..);
        } else {
            self.redo_stack.clear();
            self.undo_stack.push((
                Action {
                    id: *line_id,
                    new: line.clone(),
                    extra: ExtraAction::None,
                },
                (cx, cy)
            ));
            self.buffer_lines[cy].1.remove(cxb);
        }
    }

    pub fn home(&mut self) {
        let (cx, _) = self.cursor_pos;
        self.cursor_pos.0 = 0;
        if cx < 1 {
            self.cursor_pos.1 = 0;
        }
    }
    
    pub fn end(&mut self) {
        let (cx, cy) = self.cursor_pos;
        let current_line = self.buffer_lines[cy].1.chars().count();
        if cx < current_line {
            self.cursor_pos.0 = current_line;
        } else {
            self.cursor_pos.1 = self.buffer_lines.len() - 1;
            self.cursor_pos.0 = self.buffer_lines[self.cursor_pos.1].1.chars().count();
        }
    }
}