mod editor;

use crossterm::{cursor, event::{self, Event, KeyCode}, style, terminal};
use std::{io::Write, time::Duration};
use editor::Editor;

macro_rules! exec {
    ( $( $command:expr ),+ $(,)? ) => {
        ::crossterm::execute!(::std::io::stdout(), $( $command ),+)
    }
}

fn save<T: AsRef<str>>(path: Option<T>, contents: &str) -> anyhow::Result<()> {
    if let Some(p) = path {
        std::fs::write(p.as_ref(), contents.as_bytes())?;
    } else {
        print!("save to: ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            std::fs::write(trimmed, contents.as_bytes())?;
            println!("file saved");
            return Ok(());
        }
        loop {
            print!("do you want to save changes? [y/N] ");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            match input.trim() {
                "y" | "Y" => break,
                "n" | "N" | "" => {
                    println!("save cancelled");
                    return Ok(());
                },
                _ => continue,
            }
        }
        print!("save to: ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            std::fs::write(trimmed, contents.as_bytes())?;
            println!("file saved");
        } else {
            println!("save cancelled");
        }
    }
    Ok(())
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

    let mut save_path = None;
    let mut editor = if let Some(path) = std::env::args().nth(1) {
        save_path = Some(path.clone());
        std::fs::read_to_string(path)
            .map(|contents| Editor::from_buffer(contents.replace('\t', "    ")))
            .unwrap_or(Editor::new())
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
                        KeyCode::Char('z')
                            if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                editor.undo();
                                dirty = true;
                            },
                        KeyCode::Char('y')
                            if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                editor.redo();
                                dirty = true;
                            },
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
                        KeyCode::Delete => {
                            editor.delete_char_front();
                            dirty = true;
                        },
                        KeyCode::Home => editor.home(),
                        KeyCode::End => editor.end(),
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
                    .iter()
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

    save(save_path, &editor.get_buffer_lines().join("\n"))?;
    Ok(())
}