use crossterm::terminal;
use std::{io, io::Write};

pub struct Lines {
  left:  Vec<String>,
  right: Vec<String>,
}

impl Lines {
  pub fn new() -> Lines { Lines { left: vec![], right: vec![] } }

  pub fn push_left(&mut self, v: String) { self.left.push(v); }
  pub fn push_right(&mut self, v: String) { self.right.push(v); }

  pub fn draw(&self) -> io::Result<()> {
    let (cols, _rows) = terminal::size()?;
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    write!(writer, "\x1b[s")?; // save pos
    for i in 0..self.left.len().max(self.right.len()) {
      // go to start and clear line
      write!(writer, "\x1b[{};1H\x1b[K", i + 1)?;
      match (self.left.get(i), self.right.get(i)) {
        (Some(left), Some(right)) => {
          write!(writer, "{}", left)?;
          write!(writer, "\x1b[{};{}H", i + 1, cols - right.len() as u16 + 1)?;
          write!(writer, "{}", right)?;
        }
        (None, Some(right)) => {
          write!(writer, "\x1b[{};{}H", i + 1, cols - right.len() as u16 + 1)?;
          write!(writer, "{}", right)?;
        }
        (Some(left), None) => {
          write!(writer, "{}", left)?;
        }
        (None, None) => unreachable!(),
      }
    }
    write!(writer, "\x1b[u")?; // restore pos
    writer.flush()?;
    Ok(())
  }
}
