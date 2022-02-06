use clap::Parser;
use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyCode::Char},
    style::{self, Color, Stylize},
    terminal, ExecutableCommand, QueueableCommand,
};
use std::cmp::min;
use std::fs::{self, File};
use std::io::{self, Read, Result, Stdout, Write};
use unicode_segmentation::UnicodeSegmentation;

// only unix / darwin for now
#[cfg(unix)]
extern crate term_size;
#[cfg(unix)]
extern crate termios;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The path to the file to read
    #[clap(parse(from_os_str))]
    path: std::path::PathBuf,
}

enum Action {
    Quit,
    Up,
    Down,
    Left,
    Right,
    PageDown,
}

struct DisplayInfo {
    next_char: usize,
    line_lengths: Vec<u16>,
}

fn await_input() -> Result<Action> {
    loop {
        use Action::*;
        match read()? {
            Event::Key(event) => match event.code {
                Char('q') => return Ok(Quit),
                Char('k') => return Ok(Up),
                Char('j') => return Ok(Down),
                Char('h') => return Ok(Left),
                Char('l') => return Ok(Right),
                Char(' ') => return Ok(PageDown),
                _ => continue,
            },
            _ => continue,
        }
    }
}

// display provided string
// returns the byte index immediately after the last displayed grapheme
fn display(
    stdout: &mut Stdout,
    s: &str,
    pt: usize,
    width: u16,
    height: u16,
) -> Result<DisplayInfo> {
    let mut line = 0;
    let mut column = 0;
    let mut line_lengths = Vec::new();

    stdout
        .queue(cursor::SavePosition)?
        .queue(cursor::MoveTo(0, 0))?
        .queue(terminal::Clear(terminal::ClearType::All))?;

    let complete = |stdout: &mut Stdout, info: DisplayInfo| {
        stdout.queue(cursor::RestorePosition)?;
        stdout.flush()?;
        Ok(info)
    };

    // TODO consider word splitting here
    // need to be careful about words longer than line
    for (i, g) in s[pt..].grapheme_indices(false) {
        // TODO handle double-width chars in monospace font
        if column == width || is_newline(g) {
            line_lengths.push(column);
            column = 0;
            line += 1;
            if line == height {
                return complete(
                    stdout,
                    DisplayInfo {
                        line_lengths: line_lengths,
                        next_char: pt + i,
                    },
                );
            }
            stdout.queue(cursor::MoveToNextLine(1))?;
        }
        if !is_newline(g) {
            column += 1;
            stdout.queue(style::Print(g))?;
        }
    }
    return complete(
        stdout,
        DisplayInfo {
            next_char: s.len(),
            line_lengths: line_lengths,
        },
    );
}

// is the first character a newline of some sort
fn is_newline(s: &str) -> bool {
    match s.bytes().nth(0) {
        Some(b'\n') => true,
        Some(b'\r') => true,
        _ => false,
    }
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let mut stdout = io::stdout();
    stdout
        .queue(terminal::Clear(terminal::ClearType::All))?
        .queue(style::SetForegroundColor(Color::Red))?
        .queue(cursor::MoveTo(0, 0))?
        .flush()?;
    terminal::enable_raw_mode()?;
    let (w, h) = terminal::size()?;
    let contents = fs::read_to_string(args.path)?;
    let mut display_info = display(&mut stdout, &contents, 0, w, h)?;
    let mut target_column = 0;
    loop {
        use Action::*;
        let act = await_input()?;
        let (c, r) = cursor::position()?;
        match act {
            Quit => break,
            // TODO make these cases more uniform
            Up => {
                // TODO handle r=0
                stdout.execute(cursor::MoveTo(
                    min(target_column, display_info.line_lengths[(r - 1) as usize]),
                    r - 1,
                ))?;
            }
            Down => {
                // TODO handle r=h
                stdout.execute(cursor::MoveTo(
                    min(target_column, display_info.line_lengths[(r + 1) as usize]),
                    r + 1,
                ))?;
            }
            Right => {
                if c + 1 > display_info.line_lengths[r as usize] {
                    target_column = 0;
                    stdout.execute(cursor::MoveTo(0, r + 1))?;
                } else {
                    target_column = c + 1;
                    stdout.execute(cursor::MoveRight(1))?;
                }
            }
            Left => {
                if c == 0 {
                    target_column = display_info.line_lengths[(r - 1) as usize];
                    stdout.execute(cursor::MoveTo(target_column, r - 1))?;
                } else {
                    target_column = c - 1;
                    stdout.execute(cursor::MoveLeft(1))?;
                }
            }
            PageDown => {
                if display_info.next_char < contents.len() {
                    display_info = display(&mut stdout, &contents, display_info.next_char, w, h)?
                }
            }
        };
    }
    stdout
        .queue(terminal::Clear(terminal::ClearType::All))?
        .queue(cursor::MoveTo(0, 0))?
        .flush()?;
    terminal::disable_raw_mode()?;
    Ok(())
}
