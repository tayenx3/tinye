mod editor;

use crossterm::{cursor, event::{self, Event, KeyCode}, style, terminal};
use std::{io::Write, time::Duration};
use editor::Editor;

macro_rules! exec {
    ( $( $command:expr ),+ $(,)? ) => {
        ::crossterm::execute!(::std::io::stdout(), $( $command ),+)
    }
}

fn main() -> anyhow::Result<()> {
    terminal::enable_raw_mode()?;
    exec!(
        terminal::EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Show,
        terminal::DisableLineWrap,
    )?;

    let mut save_path = "main.txt".to_string();
    let mut editor = if let Some(path) = std::env::args().skip(1).next() {
        save_path = path.clone();
        let contents = std::fs::read_to_string(path)?;
        Editor::from_buffer(contents)
    } else {
        Editor::new()
    };

    let mut dirty = true;
    'main: loop {
        if event::poll(Duration::from_millis(10))? {
            let digit_len = editor.get_buffer_lines().len().ilog10() + 1;
            match event::read()? {
                Event::Key(key_event) if key_event.kind != event::KeyEventKind::Release => {
                    match key_event.code {
                        KeyCode::Esc => break 'main,
                        KeyCode::Right => editor.move_right(),
                        KeyCode::Left => editor.move_left(),
                        KeyCode::Up => editor.move_up(),
                        KeyCode::Down => editor.move_down(),
                        KeyCode::Enter => {
                            editor.insert_char('\n');
                            dirty = true;
                        },
                        KeyCode::Char(ch) => {
                            editor.insert_char(ch);
                            dirty = true;
                        },
                        KeyCode::Tab => {
                            editor.insert_str("    ");
                            dirty = true;
                        },
                        KeyCode::Backspace => {
                            editor.delete_char();
                            dirty = true;
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
            if dirty {
                exec!(
                    terminal::Clear(terminal::ClearType::All),
                    cursor::SavePosition
                )?;
                for (line_idx, line)
                in editor.get_buffer_lines()
                    .into_iter()
                    .enumerate()
                {
                    exec!(
                        cursor::MoveTo(0, line_idx as u16),
                        style::SetForegroundColor(style::Color::Cyan),
                        style::SetBackgroundColor(style::Color::DarkGrey),
                        style::Print(format!(" {:>w$} ", line_idx + 1, w = digit_len as usize)),
                        style::SetForegroundColor(style::Color::Black),
                        style::Print("│"),
                        style::SetForegroundColor(style::Color::Reset),
                        style::SetBackgroundColor(style::Color::Reset),
                        style::Print(" "),
                        style::Print(line),
                    )?;
                }
                exec!(cursor::RestorePosition)?;
                dirty = false;
            }
            let pos = editor.get_pos();
            exec!(cursor::MoveTo(pos.0 as u16 + digit_len as u16 + 4, pos.1 as u16))?;
        }
    }
    exec!(terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    
    print!("save to (default `main.txt` or input file): ");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    save_path = match input.trim() {
        "" => save_path,
        other => other.to_string(),
    };
    std::fs::write(save_path, editor.get_buffer_lines().join("\n").as_bytes())?;
    
    Ok(())
}