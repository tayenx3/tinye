mod editor;
mod colors;

use crossterm::{cursor, event::{self, Event, KeyCode}, style, terminal};
use std::{io::Write, time::Duration};
use editor::Editor;
use clap::Parser;
use colors::ColorScheme;

macro_rules! exec {
    ( $( $command:expr ),+ $(,)? ) => {
        ::crossterm::execute!(::std::io::stdout(), $( $command ),+)
    }
}

#[derive(Parser)]
#[command(
    name = "tinye",
    about = "a lightweight terminal-based code editor written in Rust",
    version,
    author,
)]
pub struct Cli {
    input: Option<String>,
    #[arg(
        short,
        long,
        help = "Color theme"
    )]
    theme: Option<ColorScheme>
}

fn render_status_bar<P: AsRef<str>>(
    file_name: Option<P>,
    term_size: (u16, u16),
    cursor_pos: (usize, usize),
    theme: ColorScheme
) -> anyhow::Result<()> {
    exec!(
        cursor::SavePosition,
        cursor::MoveTo(0, term_size.1.saturating_sub(1))
    )?;
    let mut file_name_len = 0;
    if let Some(p) = file_name {
        file_name_len = p.as_ref().len() + 1;
        let mut fmt = format!(" {} ", p.as_ref());
        fmt.truncate(Editor::char_to_byte(term_size.0 as usize - 1, &fmt));
        exec!(
            style::SetForegroundColor(theme.status_bar_bg),
            style::SetBackgroundColor(theme.status_bar_fg),
            style::Print(fmt),
        )?;
    }
    let mut pos_fmt = format!(
        " {}:{} ",
        cursor_pos.1 + 1, // line
        cursor_pos.0 + 1, // col
    );
    pos_fmt.truncate(Editor::char_to_byte(
        (term_size.0 as usize).saturating_sub(file_name_len + 1),
        &pos_fmt
    ));
    let sb_len = file_name_len + pos_fmt.len();
    exec!(
        style::SetForegroundColor(theme.status_bar_fg),
        style::SetBackgroundColor(theme.status_bar_bg),
        style::Print(pos_fmt),
        style::Print(" ".repeat((term_size.0 as usize).saturating_sub(sb_len))),
        style::SetForegroundColor(theme.fg),
        style::SetBackgroundColor(theme.bg),
        cursor::RestorePosition,
    )?;
    Ok(())
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
    let cli = Cli::parse();
    let theme = cli.theme.unwrap_or(ColorScheme::DEFAULT);
    
    terminal::enable_raw_mode()?;
    exec!(
        terminal::EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Show,
        terminal::DisableLineWrap,
    )?;

    let mut term_size = terminal::size()?;
    let mut save_path = None;
    let mut editor = if let Some(path) = cli.input {
        save_path = Some(path.clone());
        std::fs::read_to_string(path)
            .map(|contents| Editor::from_buffer(contents.replace('\t', "    ")))
            .unwrap_or(Editor::new())
    } else {
        Editor::new()
    };

    // this just means if the VIEW BUFFER is dirty, not the actual editor buffer
    let mut dirty = true;
    'main: loop {
        if event::poll(Duration::from_millis(10))? {
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
                        KeyCode::Up => if key_event.modifiers.contains(event::KeyModifiers::ALT) {
                            editor.scroll_up(5);
                            dirty = true;
                        } else if key_event.modifiers.contains(event::KeyModifiers::CONTROL) {
                            editor.scroll_up(1);
                            dirty = true;
                        } else {
                            editor.move_up();
                        },
                        KeyCode::Down => if key_event.modifiers.contains(event::KeyModifiers::ALT) {
                            editor.scroll_down(5);
                            dirty = true;
                        } else if key_event.modifiers.contains(event::KeyModifiers::CONTROL) {
                            editor.scroll_down(1);
                            dirty = true;
                        } else {
                            editor.move_down();
                        },
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
                        KeyCode::PageUp => {
                            editor.scroll_up(term_size.1 as usize - 1);
                            dirty = true;
                        },
                        KeyCode::PageDown => {
                            editor.scroll_down(term_size.1 as usize - 1);
                            dirty = true;
                        },
                        _ => {}
                    }
                },
                Event::Resize(nx, ny) => {
                    term_size = (nx, ny);
                    dirty = true;
                },
                _ => {}
            }
            let sc_amt = editor.get_scroll_amount();
            let buffer_lines = editor.get_visible_buffer_lines(term_size.1);
            let digit_len = (buffer_lines.len().max(1) + sc_amt).ilog10() as usize + 1;
            if dirty {
                exec!(
                    terminal::Clear(terminal::ClearType::All),
                    cursor::SavePosition
                )?;
                let mut lines = buffer_lines.iter().enumerate();
                let mut line_idx = 0..(term_size.1 as usize).saturating_sub(1);
                while let (Some(idx), line) = (line_idx.next(), lines.next()) {
                    exec!(
                        cursor::MoveTo(0, idx as u16),
                        style::SetBackgroundColor(theme.gutter_bg)
                    )?;
                    match line {
                        Some((line_idx, _)) => exec!(
                            style::SetForegroundColor(theme.line_num_fg),
                            style::Print(format!(" {:>w$} ", line_idx + sc_amt + 1, w = digit_len as usize))
                        )?,
                        None => exec!(
                            style::SetForegroundColor(theme.gutter_fg),
                            style::Print(format!(" {:>w$} ", "~", w = digit_len as usize))
                        )?,
                    }
                    exec!(
                        style::SetForegroundColor(theme.gutter_fg),
                        style::Print("│"),
                        style::SetForegroundColor(theme.fg),
                        style::SetBackgroundColor(theme.bg),
                        style::Print(" "),
                    )?;
                    if let Some((_, (_, line))) = line {
                        if line.chars().count() < term_size.0 as usize {
                            exec!(style::Print(format!(
                                "{}{}",
                                line,
                                " ".repeat(term_size.0 as usize - line.len())
                            )))?;
                        } else {
                            exec!(style::Print(
                                line.chars()
                                    .take(term_size.0 as usize)
                                    .collect::<String>()
                            ))?;
                        }
                    } else {
                        exec!(style::Print(" ".repeat(term_size.0 as usize)))?;
                    }
                }
                exec!(cursor::RestorePosition)?;
                dirty = false;
            }
            let pos = editor.get_pos();
            render_status_bar(save_path.as_ref(), term_size, pos, theme)?;
            if pos.1 >= sc_amt && pos.1 <= (term_size.1 as usize - 2 + sc_amt) {
                exec!(
                    cursor::MoveTo(
                        pos.0 as u16 + digit_len as u16 + 4,
                        (pos.1 as u16).saturating_sub(sc_amt as u16)
                    ),
                    cursor::Show
                )?;
            } else {
                exec!(cursor::Hide)?;
            }
        }
    }
    exec!(terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    save(save_path, &editor.get_full_buffer())?;
    Ok(())
}