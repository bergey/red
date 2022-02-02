use clap::Parser;
use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyCode::Char},
    style::{self, Color, Stylize},
    terminal, ExecutableCommand, QueueableCommand,
};
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
                _ => continue,
            },
            _ => continue,
        }
    }
}

// display provided string
// returns the byte index immediately after the last displayed grapheme
fn display(stdout: &mut Stdout, s: &str, width: u16, height: u16) -> Result<DisplayInfo> {
    let mut line = 0;
    let mut column = 0;
    let mut line_lengths = Vec::new();

    stdout.queue(cursor::SavePosition)?;

    let complete = |stdout: &mut Stdout, info: DisplayInfo| {
        stdout.queue(cursor::RestorePosition)?;
        stdout.flush()?;
        Ok(info)
    };

    // TODO consider word splitting here
    // need to be careful about words longer than line
    for (i, g) in s.grapheme_indices(false) {
        // TODO handle double-width chars in monospace font
        if column == width || is_newline(g) {
            line_lengths.push(column);
            column = 0;
            line += 1;
            if line == height {
                return complete(stdout, DisplayInfo {
                    line_lengths: line_lengths,
                    next_char: i
                });
            }
            stdout.queue(cursor::MoveToNextLine(1))?;
        }
        if !is_newline(g) {
            column += 1;
            stdout.queue(style::Print(g))?;
        }
    }
    return complete(stdout, DisplayInfo {
        next_char: s.len(),
        line_lengths: line_lengths
    });
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
    let display_info = display(&mut stdout, &contents, w, h)?;
    loop {
        use Action::*;
        let act = await_input()?;
        match act {
            Quit => break,
            Up => {
                stdout.execute(cursor::MoveUp(1))?;
            },
            Down => {
                stdout.execute(cursor::MoveDown(1))?;
            },
            Right => {
                let (c, r) = cursor::position()?;
                if c+1 > display_info.line_lengths[r as usize] {
                    stdout.execute(cursor::MoveTo(0, r+1))?;
                } else {
                    stdout.execute(cursor::MoveRight(1))?;
                }
            },
            Left => {
                let (c, r) = cursor::position()?;
                if c == 0 {
                    stdout.execute(cursor::MoveTo(display_info.line_lengths[(r-1) as usize], r-1))?;
                } else {
                    stdout.execute(cursor::MoveLeft(1))?;
                }
            },
        };
    }
    stdout
        .queue(terminal::Clear(terminal::ClearType::All))?
        .queue(cursor::MoveTo(0, 0))?
        .flush()?;
    terminal::disable_raw_mode()?;
    Ok(())
}
