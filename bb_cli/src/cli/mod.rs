use crossterm::{execute, terminal};
use log::Record;
use log4rs::{
  append::Append,
  config::Appender,
  encode::{writer::ansi::AnsiWriter, Encode},
};
use std::{collections::VecDeque, io, io::Write, sync::Mutex};

mod line;

#[derive(Debug)]
pub struct ScrollBuf {
  min:     u16,
  len:     u16,
  buf:     VecDeque<u8>,
  restore: bool,
}

impl ScrollBuf {
  pub fn new(min: u16, len: u16) -> ScrollBuf {
    ScrollBuf { min, len, buf: VecDeque::new(), restore: true }
  }
  pub fn new_no_restore(min: u16, len: u16) -> ScrollBuf {
    ScrollBuf { min, len, buf: VecDeque::new(), restore: false }
  }
}

impl io::Write for ScrollBuf {
  fn write(&mut self, data: &[u8]) -> io::Result<usize> {
    self.buf.extend(data);
    if data.contains(&b'\n') {
      self.flush()?;
    }
    Ok(data.len())
  }
  fn flush(&mut self) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    if self.restore {
      write!(writer, "\x1b[s")?; // save pos
    }
    let mut line = 0;
    let mut idx = 0;
    for (i, &c) in self.buf.iter().enumerate().rev() {
      if c == b'\n' {
        line += 1;
      }
      if line > self.len {
        idx = i + 1;
        break;
      }
    }
    self.buf.drain(0..idx);
    let mut line = 0;
    write!(writer, "\x1b[{};1H\x1b[K", self.min)?; // go to start, erase line
    for (i, &c) in self.buf.iter().enumerate() {
      if c == b'\n' {
        if self.buf.get(i + 1).is_some() {
          line += 1;
          write!(writer, "\x1b[{};1H\x1b[K", line + self.min)?;
          // go to start, erase line
        }
      } else {
        writer.write_all(&[c])?;
      }
    }
    if self.restore {
      write!(writer, "\x1b[u")?; // restore pos
    }
    writer.flush()?;
    Ok(())
  }
}

impl ScrollBuf {
  pub fn back(&mut self) -> io::Result<()> {
    self.buf.pop_back();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    write!(writer, "\x1b[1D \x1b[1D")?; // left 1 char, print space, left 1 char
    writer.flush()?;

    Ok(())
  }
  pub fn buf(&mut self) -> &mut VecDeque<u8> { &mut self.buf }
}

pub fn skip_appender(min: u16, len: u16) -> impl io::Write { ScrollBuf::new(min, len) }

pub fn setup() -> Result<(), io::Error> {
  let stdout = io::stdout();
  let mut w = stdout.lock();

  terminal::enable_raw_mode()?;
  execute!(io::stdout(), terminal::EnterAlternateScreen)?;

  write!(w, "\x1b[2J")?; // clear
  Ok(())
}

pub struct LineReader {
  buf:    ScrollBuf,
  prompt: &'static str,
}

impl LineReader {
  pub fn new(prompt: &'static str, min: u16, len: u16) -> Self {
    LineReader { buf: ScrollBuf::new_no_restore(min, len), prompt }
  }

  pub fn read_line(&mut self) -> Result<String, io::Error> {
    line::SingleLineReader::new(&mut self.buf, self.prompt).read()
  }
}

impl io::Write for LineReader {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.buf.write(buf) }
  fn flush(&mut self) -> io::Result<()> { self.buf.flush() }
}
