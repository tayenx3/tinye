mod editor;
mod cmdp;
mod colors;

mod utils {
    pub fn char_to_byte(pos: usize, s: &str) -> usize {
        s.chars().take(pos).map(|c| c.len_utf8()).sum()
    }
}

use crossterm::{cursor, event::{self, Event, KeyCode}, style, terminal};
use std::{
    io::Write as _,
    time::Duration,
    fmt::Write as _,
    process::{Command, Stdio},
};
use editor::Editor;
use clap::Parser;
use colors::ColorScheme;
use std::path::PathBuf;

macro_rules! exec {
    ( $( $command:expr ),+ $(,)? ) => {
        ::crossterm::queue!(::std::io::stdout(), $( $command ),+)
    };
    () => {
        <::std::result::Result<(), std::io::Error>>::Ok(())
    };
}

macro_rules! flush {
    () => {
        ::std::io::stdout().flush()
    };
}

#[derive(Parser)]
#[command(
    name = "tinye",
    about = "a lightweight terminal-based code editor written in Rust",
    version = concat!(env!("CARGO_PKG_VERSION"), "-alpha.1"),
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
    in_editor: bool,
    file_name: Option<P>,
    term_size: (u16, u16),
    cursor_pos: (usize, usize),
    theme: ColorScheme
) -> anyhow::Result<()> {
    exec!(
        cursor::Hide,
        cursor::SavePosition,
        cursor::MoveTo(0, term_size.1.saturating_sub(1))
    )?;
    let mut file_name_len = 0;
    if let Some(p) = file_name {
        file_name_len = p.as_ref().chars().count();
        let mut fmt = format!(
            " {} ", 
            PathBuf::from(p.as_ref())
                .file_name()
                .map(|p| p.display().to_string())
                .unwrap_or(p.as_ref().to_string())
        );
        fmt.truncate(utils::char_to_byte(term_size.0 as usize - 1, &fmt));
        exec!(
            style::SetForegroundColor(theme.status_bar_bg),
            style::SetBackgroundColor(theme.status_bar_fg),
            style::Print(fmt),
        )?;
    }
    let mut pos_fmt = format!(
        " {}:{}  {}",
        cursor_pos.1 + 1, // line
        cursor_pos.0 + 1, // col
        in_editor.then(|| "EDITOR").unwrap_or("CMD")
    );
    let pos_trunc = (term_size.0 as usize).saturating_sub(file_name_len);
    pos_fmt.truncate(utils::char_to_byte(pos_trunc, &pos_fmt));
    let sb_len = file_name_len + pos_fmt.chars().count();
    exec!(
        style::SetForegroundColor(theme.status_bar_fg),
        style::SetBackgroundColor(theme.status_bar_bg),
        style::Print(pos_fmt),
        style::Print(" ".repeat((term_size.0 as usize).saturating_sub(sb_len))),
        style::SetForegroundColor(theme.fg),
        style::SetBackgroundColor(theme.bg),
        cursor::RestorePosition,
        cursor::Show,
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

    let mut prev_files = Vec::new();
    let mut in_editor = true;
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
    let mut cmdp = cmdp::CommandPalette::new();
    let mut line_buf = String::with_capacity(100);
    let mut space_buf = " ".repeat(term_size.0 as usize);
    let mut dirty = true;
    let mut cursor_moved = true;
    let mut last_render = std::time::Instant::now();
    const RENDER_INTERVAL: Duration = Duration::from_millis(16);
    'main: loop {
        while event::poll(Duration::from_millis(0))? {
            if in_editor {
                match event::read()? {
                    Event::Key(key_event) if key_event.kind != event::KeyEventKind::Release => {
                        match key_event.code {
                            KeyCode::Esc => break 'main,
                            KeyCode::Char('p')
                                if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                    in_editor = false;
                                    cursor_moved = true;
                                    dirty = true;
                                },
                            KeyCode::Char('z')
                                if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                    editor.undo();
                                    dirty = true;
                                    cursor_moved = true;
                                },
                            KeyCode::Char('y')
                                if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                    editor.redo();
                                    dirty = true;
                                    cursor_moved = true;
                                },
                            KeyCode::Right => {
                                editor.move_right();
                                cursor_moved = true;
                            },
                            KeyCode::Left => {
                                editor.move_left();
                                cursor_moved = true;
                            },
                            KeyCode::Up => if key_event.modifiers.contains(event::KeyModifiers::ALT) {
                                editor.scroll_up(5);
                                dirty = true;
                                cursor_moved = true;
                            } else if key_event.modifiers.contains(event::KeyModifiers::CONTROL) {
                                editor.scroll_up(1);
                                dirty = true;
                                cursor_moved = true;
                            } else {
                                editor.move_up();
                                cursor_moved = true;
                            },
                            KeyCode::Down => if key_event.modifiers.contains(event::KeyModifiers::ALT) {
                                editor.scroll_down(5);
                                dirty = true;
                                cursor_moved = true;
                            } else if key_event.modifiers.contains(event::KeyModifiers::CONTROL) {
                                editor.scroll_down(1);
                                dirty = true;
                                cursor_moved = true;
                            } else {
                                editor.move_down();
                                cursor_moved = true;
                            },
                            KeyCode::Enter => {
                                editor.insert_char('\n');
                                dirty = true;
                                cursor_moved = true;
                            },
                            KeyCode::Char(ch) => {
                                editor.insert_char(ch);
                                dirty = true;
                                cursor_moved = true;
                            },
                            KeyCode::Tab => {
                                editor.insert_str("    ");
                                dirty = true;
                                cursor_moved = true;
                            },
                            KeyCode::Backspace => {
                                editor.delete_char();
                                dirty = true;
                                cursor_moved = true;
                            },
                            KeyCode::Delete => {
                                editor.delete_char_front();
                                dirty = true;
                            },
                            KeyCode::Home => {
                                editor.home();
                                cursor_moved = true;
                            },
                            KeyCode::End => {
                                editor.end();
                                cursor_moved = true;
                            },
                            KeyCode::PageUp => {
                                editor.scroll_up(term_size.1 as usize - 1);
                                dirty = true;
                                cursor_moved = true;
                            },
                            KeyCode::PageDown => {
                                editor.scroll_down(term_size.1 as usize - 1);
                                dirty = true;
                                cursor_moved = true;
                            },
                            _ => {}
                        }
                    },
                    Event::Resize(nx, ny) => {
                        term_size = (nx, ny);
                        space_buf = " ".repeat(nx as usize);
                        dirty = true;
                        cursor_moved = true;
                    },
                    _ => {}
                }
            } else {
                match event::read()? {
                    Event::Key(key_event) if key_event.kind != event::KeyEventKind::Release => {
                        match key_event.code {
                            KeyCode::Esc => {
                                in_editor = true;
                                cursor_moved = true;
                                dirty = true;
                            },
                            KeyCode::Char('z')
                                if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                    cmdp.undo();
                                    dirty = true;
                                    cursor_moved = true;
                                },
                            KeyCode::Char('y')
                                if key_event.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                    cmdp.redo();
                                    dirty = true;
                                    cursor_moved = true;
                                },
                            KeyCode::Right => {
                                cmdp.move_right();
                                cursor_moved = true;
                            },
                            KeyCode::Left => {
                                cmdp.move_left();
                                cursor_moved = true;
                            },
                            KeyCode::Char(ch) => {
                                cmdp.insert_char(ch);
                                cursor_moved = true;
                                dirty = true;
                            },
                            KeyCode::Backspace => {
                                cmdp.delete_char();
                                cursor_moved = true;
                                dirty = true;
                            },
                            KeyCode::Delete => {
                                cmdp.delete_char_front();
                                cursor_moved = true;
                                dirty = true;
                            },
                            KeyCode::Enter => {
                                let full_command = cmdp.take_command();
                                for command in full_command.split(";") {
                                    let mut args = command.split_whitespace();
                                    match args.next() {
                                        Some("tm") => {
                                            let cmd_args = args.collect::<Vec<_>>();
                                            if !cmd_args.is_empty() {
                                                let output = if cfg!(windows) {
                                                    Command::new("powershell")
                                                        .args(&["-Command", &cmd_args.join(" ")])
                                                        .stdout(Stdio::piped())
                                                        .stderr(Stdio::piped())
                                                        .output()?
                                                } else {
                                                    Command::new("sh")
                                                        .args(&["-c", &cmd_args.join(" ")])
                                                        .stdout(Stdio::piped())
                                                        .stderr(Stdio::piped())
                                                        .output()?
                                                };
                                                std::fs::write(
                                                    format!("{}_out.txt", cmd_args[0]),
                                                    output.stdout
                                                )?;
                                                std::fs::write(
                                                    format!("{}_err.txt", cmd_args[0]),
                                                    output.stderr
                                                )?;
                                            }
                                        },
                                        Some("switchto") | Some("st") => {
                                            if let Some(path) = &save_path {
                                                prev_files.push(path.to_string());
                                            }
                                            editor = if let Some(path) = args.next() {
                                                if let Some(path) = args.next() {
                                                    std::fs::write(path, editor.get_full_buffer())?;
                                                } else if let Some(path) = std::mem::take(&mut save_path) {
                                                    std::fs::write(path, editor.get_full_buffer())?;
                                                }
                                                save_path = Some(path.to_string());
                                                std::fs::read_to_string(path)
                                                    .map(|contents| Editor::from_buffer(contents.replace('\t', "    ")))
                                                    .unwrap_or(Editor::new())
                                            } else {
                                                return Ok(());
                                            };
                                        },
                                        Some("switchnosave") | Some("sns") => {
                                            if let Some(path) = &save_path {
                                                prev_files.push(path.to_string());
                                            }
                                            editor = if let Some(path) = args.next() {
                                                save_path = Some(path.to_string());
                                                std::fs::read_to_string(path)
                                                    .map(|contents| Editor::from_buffer(contents.replace('\t', "    ")))
                                                    .unwrap_or(Editor::new())
                                            } else {
                                                return Ok(());
                                            };
                                        },
                                        Some("savefile") | Some("sf") => if let Some(path) = args.next() {
                                            save_path = Some(path.to_string());
                                            std::fs::write(path, editor.get_full_buffer())?;
                                        } else if let Some(path) = &save_path {
                                            std::fs::write(path, editor.get_full_buffer())?;
                                        },
                                        Some("return") | Some("ret") => if let Some(file) = prev_files.pop() {
                                            if let Some(path) = args.next() {
                                                std::fs::write(path, editor.get_full_buffer())?;
                                            } else if let Some(path) = &save_path {
                                                std::fs::write(path, editor.get_full_buffer())?;
                                            }
                                            save_path = Some(file.clone());
                                            editor = std::fs::read_to_string(file)
                                                .map(|contents| Editor::from_buffer(contents.replace('\t', "    ")))
                                                .unwrap_or(Editor::new());
                                        },
                                        Some("returnnosave") | Some("rns") => if let Some(file) = prev_files.pop() {
                                            save_path = Some(file.clone());
                                            editor = std::fs::read_to_string(file)
                                                .map(|contents| Editor::from_buffer(contents.replace('\t', "    ")))
                                                .unwrap_or(Editor::new());
                                        },
                                        Some("quit") | Some("q") => {
                                            if let Some(path) = args.next() {
                                                save_path = Some(path.to_string());
                                            }
                                            break 'main;
                                        },
                                        Some("quitnosave") | Some("qns") => {
                                            exec!(terminal::LeaveAlternateScreen, cursor::Show)?;
                                            return Ok(());
                                        },
                                        Some(_) | None => (),
                                    }
                                }
                                cursor_moved = true;
                                dirty = true;
                                in_editor = true;
                            },
                            _ => {}
                        }
                    },
                    Event::Resize(nx, ny) => {
                        term_size = (nx, ny);
                        space_buf = " ".repeat(nx as usize);
                        dirty = true;
                        cursor_moved = true;
                    },
                    _ => {}
                }
            }
        }
        let sc_amt = editor.get_scroll_amount();
        let buffer_lines = editor.get_visible_buffer_lines(term_size.1);
        let digit_len = (buffer_lines.len().max(1) + sc_amt).ilog10() as usize + 1;
        if dirty && last_render.elapsed() >= RENDER_INTERVAL {
            exec!(
                cursor::Hide,
                cursor::SavePosition,
            )?;
            let mut lines = buffer_lines.iter().enumerate();
            let mut line_idx = 0..(term_size.1 as usize).saturating_sub(1);
            while let (Some(idx), line) = (line_idx.next(), lines.next()) {
                if !in_editor
                    && idx < (cmdp.get_cursor() / term_size.0 as usize) + 1
                {
                    continue;
                }
                exec!(
                    cursor::MoveTo(0, idx as u16),
                    style::SetBackgroundColor(theme.gutter_bg)
                )?;
                line_buf.clear();
                match line {
                    Some((line_idx, _)) => {
                        write!(line_buf, " {:>w$} ", line_idx + sc_amt + 1, w = digit_len as usize)?;
                        exec!(
                            style::SetForegroundColor(theme.line_num_fg),
                            style::Print(&line_buf)
                        )?;
                    },
                    None => {
                        write!(line_buf, " {:>w$} ", "~", w = digit_len as usize)?;
                        exec!(
                            style::SetForegroundColor(theme.gutter_fg),
                            style::Print(&line_buf)
                        )?;
                    },
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
                        line_buf.clear();
                        line_buf.push_str(line);
                        let padding = term_size.0 as usize - line.chars().count();
                        if padding > 0 {
                            line_buf.push_str(&space_buf[0..padding]);
                        }
                        exec!(style::Print(&line_buf))?;
                    } else {
                        line_buf.clear();
                        let chars_to_take = term_size.0 as usize;
                        line_buf.extend(line.chars().take(chars_to_take));
                        exec!(style::Print(&line_buf))?;
                    }
                } else {
                    exec!(style::Print(&space_buf[(digit_len + 4)..term_size.0 as usize]))?;
                }
            }
            exec!(cursor::RestorePosition, cursor::Show)?;
            dirty = false;
            last_render = std::time::Instant::now();
        }
        let pos = editor.get_pos();
        if cursor_moved {
            render_status_bar(in_editor, save_path.as_ref(), term_size, pos, theme)?;
            if in_editor {
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
                cursor_moved = false;
            }
        } else if in_editor {
            if pos.1 >= sc_amt && pos.1 <= (term_size.1 as usize - 2 + sc_amt) {
                exec!(cursor::Show)?;
            } else {
                exec!(cursor::Hide)?;
            }
        }
        if !in_editor {
            exec!(
                cursor::Hide,
                cursor::MoveTo(0, 0),
                style::SetBackgroundColor(theme.status_bar_bg),
                style::SetForegroundColor(theme.status_bar_fg),
                style::Print("> "),
            )?;
            let c = cmdp.get_command();
            let cmd_len = c.chars().count();
            let tsizex = term_size.0 as usize;
            if cmd_len <= tsizex - 2 {
                exec!(style::Print(&c), style::Print(&space_buf[cmd_len..tsizex - 2]))?;
            } else {
                exec!(style::Print(&c[..(tsizex - 2)]))?;
                let mut cx = tsizex - 2;
                let mut cy = 1;
                while cmd_len > cx {
                    exec!(
                        cursor::MoveTo(0, cy),
                        style::Print(trunc(&c[cx..], tsizex))
                    )?;
                    cx += tsizex;
                    cy += 1;
                }
                exec!(style::Print(&space_buf[..(cmd_len % tsizex)]))?;
            }
            let cmx = cmdp.get_cursor() + 2;
            exec!(
                cursor::Show,
                cursor::MoveTo((cmx % tsizex) as u16, (cmx / tsizex) as u16),
                style::SetBackgroundColor(theme.bg),
                style::SetForegroundColor(theme.fg),
            )?;
        }
        
        flush!()?;
    }
    exec!(terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;

    save(save_path, &editor.get_full_buffer())?;
    Ok(())
}

#[inline]
fn trunc(s: &str, pos: usize) -> &str {
    if s.chars().count() <= pos {
        s
    } else {
        &s[..pos]
    }
}