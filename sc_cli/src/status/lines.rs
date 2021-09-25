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
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    writer.write(b"\x1b[s")?; // save pos
    for (i, l) in self.left.iter().enumerate() {
      writer.write(format!("\x1b[{};1H\x1b[K", i + 1).as_bytes())?; // go to start and clear line
      writer.write(l.as_bytes())?;
    }
    writer.write(b"\x1b[u")?; // restore pos
    Ok(())
  }
}
