use log::Record;
use log4rs::{
  append::Append,
  config::Appender,
  encode::{writer::ansi::AnsiWriter, Encode},
};
use std::{
  io,
  io::{Stdout, Write},
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
}

impl Append for SkipConsoleAppender {
  fn append(&self, record: &Record) -> anyhow::Result<()> {
    let mut writer = AnsiWriter(self.writer.lock());
    writer.write(b"\x1b[5A")?; // up a line, then insert a newline
    self.encoder.encode(&mut writer, record)?;
    writer.write(b"\n\x1b[5B")?;
    // writer.write(b"\n")?; // newline
    // writer.write(b"\x1b[1A")?; // up 1 line
    // writer.write(b"\x1b[u")?; // restore cursor
    writer.flush()?;
    Ok(())
  }

  fn flush(&self) {}
}

impl SkipConsoleAppender {
  /// Creates a new `ConsoleAppender` builder.
  pub fn new<E: Encode>(skip: usize, encoder: E) -> SkipConsoleAppender {
    SkipConsoleAppender { skip, writer: io::stdout(), encoder: Box::new(encoder) }
  }
}

pub fn skip_appender(skip: usize) -> Appender {
  Appender::builder()
    .build("stdout", Box::new(SkipConsoleAppender::new(skip, sc_common::make_pattern())))
}
