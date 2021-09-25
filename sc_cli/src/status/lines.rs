use crossterm::terminal;
use std::{io, io::Write};

pub struct Lines {
  left:  Vec<String>,
  right: Vec<String>,
}

impl Lines {
  pub fn new() -> Lines {
    Lines { left: vec![], right: vec![] }
  }

  pub fn push_left(&mut self, v: String) {
    self.left.push(v);
  }
  pub fn push_right(&mut self, v: String) {
    self.right.push(v);
  }

  pub fn draw(&self) -> io::Result<()> {
    let (cols, _rows) = terminal::size()?;
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    writer.write(b"\x1b[s")?; // save pos
    for i in 0..self.left.len().max(self.right.len()) {
      // go to start and clear line
      writer.write(format!("\x1b[{};1H\x1b[K", i + 1).as_bytes())?;
      match (self.left.get(i), self.right.get(i)) {
        (Some(left), Some(right)) => {
          writer.write(left.as_bytes())?;
          writer.write(format!("\x1b[{};{}H", i + 1, cols - right.len() as u16 + 1).as_bytes())?;
          writer.write(right.as_bytes())?;
        }
        (None, Some(right)) => {
          writer.write(format!("\x1b[{};{}H", i + 1, cols - right.len() as u16 + 1).as_bytes())?;
          writer.write(right.as_bytes())?;
        }
        (Some(left), None) => {
          writer.write(left.as_bytes())?;
        }
        (None, None) => unreachable!(),
      }
    }
    writer.write(b"\x1b[u")?; // restore pos
    writer.flush()?;
    Ok(())
  }
}
