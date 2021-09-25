use log::Record;
use log4rs::{
  append::Append,
  config::Appender,
  encode::{writer::ansi::AnsiWriter, Encode},
};
use std::{
  io,
  io::{Stdout, Write},
  sync::Mutex,
};

/// An appender which logs to standard out.
///
/// It supports output styling if standard out is a console buffer on Windows
/// or is a TTY on Unix.
#[derive(Debug)]
pub struct SkipConsoleAppender {
  skip:    usize,
  writer:  Stdout,
  encoder: Box<dyn Encode>,
  buf:     Mutex<Vec<u8>>,
}

impl Append for SkipConsoleAppender {
  fn append(&self, record: &Record) -> anyhow::Result<()> {
    let mut writer = AnsiWriter(self.writer.lock());
    let mut buf = self.buf.lock().unwrap();
    self.encoder.encode(&mut AnsiWriter(&mut buf as &mut Vec<u8>), record)?;
    writer.write(b"\x1b[s")?; // save pos
    writer.write(b"\x1b[15;1H")?; // go to start
    let mut line = 0;
    let mut idx = 0;
    for (i, &c) in buf.iter().enumerate().rev() {
      if c == b'\n' {
        line += 1;
      }
      if line > 30 {
        idx = i + 1;
        break;
      }
    }
    buf.drain(0..idx);
    writer.write(&buf)?; // write buf
    writer.write(b"\x1b[u")?; // restore pos
    writer.flush()?;
    Ok(())
  }

  fn flush(&self) {}
}

impl SkipConsoleAppender {
  /// Creates a new `ConsoleAppender` builder.
  pub fn new<E: Encode>(skip: usize, encoder: E) -> SkipConsoleAppender {
    SkipConsoleAppender {
      skip,
      writer: io::stdout(),
      encoder: Box::new(encoder),
      buf: Vec::new().into(),
    }
  }
}

pub fn skip_appender(skip: usize) -> Appender {
  Appender::builder()
    .build("stdout", Box::new(SkipConsoleAppender::new(skip, sc_common::make_pattern())))
}

pub fn setup() -> Result<(), io::Error> {
  let stdout = io::stdout();
  let mut w = stdout.lock();

  w.write(b"\x1b[2J")?; // clear
  Ok(())
}

pub fn draw() -> Result<(), io::Error> {
  Ok(())
}
