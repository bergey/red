use clap::Parser;

use std::fs::File;
use std::io::{self, Write, Read};
use std::os::unix::io::AsRawFd;

// only unix / darwin for now
#[cfg(unix)]
extern crate termios;
#[cfg(unix)]
extern crate term_size;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// The path to the file to read
    #[clap(parse(from_os_str))]
    path: std::path::PathBuf,
}

#[cfg(unix)]
fn setup_term() -> File {
    use termios::*;

    let tty = File::open("/dev/tty").unwrap();
    let mut term = Termios::from_fd(tty.as_raw_fd()).unwrap(); // Unix only
    // Unset canonical mode, so we get characters immediately
    // Disable local echo
    term.c_lflag &= !(ICANON | ECHO);
    tcsetattr(tty.as_raw_fd(), TCSADRAIN, &term).unwrap();
    tty
}

fn pager<R : Read, W : Write>(reader :  &mut R, writer : &mut W ) {
    let mut buffer = [0; 1024];

    let (term_columns, term_lines) = match term_size::dimensions() {
        Some((w, h)) => (w, h-1),
        None => (80, 30)
    };

    let mut want_lines = term_lines;  // start with a full page; count down
    let mut columns = term_columns;   // for consistency, count down

    'chunks: while let Ok(size) = reader.read(&mut buffer) {
        if size == 0 {
            break;
        }
        let mut write_start = 0;    // start of next write
        let mut point = 0;          // next char when counting lines

            writer.flush().unwrap();

        loop {
            // find a subrange with the right number of lines
            while want_lines > 0 {
                let c = buffer[point];
                if c == b'\n' {
                    want_lines -= 1;
                    columns = term_columns;
                    point += 1;
                }
                else if columns == 0 {
                    // visual line, wrapped by terminal
                    want_lines -= 1;
                    columns = term_columns;
                    // don't increment point; this char needs to start the next line
                } else {
                    point += 1;
                    columns -= 1;
                }
                if point == size {
                    writer.write(&buffer[write_start..point]).unwrap();
                    continue 'chunks
                }
            }

            writer.write(&buffer[write_start..point]).unwrap();
            writer.flush().unwrap();
            write_start = point;

            let tty = setup_term();
            for byte in tty.bytes() {
                match byte.unwrap() {
                    b'q' | 27 => {
                        break 'chunks;
                    }
                    _ => (),
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let args = Cli::parse();
    let stdout = io::stdout();
    let mut out_lock = stdout.lock();
    let mut f = File::open(args.path)?;
    pager(&mut f, &mut out_lock);
    Ok(())
}
