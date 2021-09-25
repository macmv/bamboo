use crossterm::{execute, terminal};
use log::Record;
use log4rs::{
  append::Append,
  config::Appender,
  encode::{writer::ansi::AnsiWriter, Encode},
};
use std::{
  collections::VecDeque,
  io,
  io::{Read, Write},
  sync::Mutex,
};

#[derive(Debug)]
pub struct ScrollBuf {
  buf: VecDeque<u8>,
}

impl ScrollBuf {
  pub fn new() -> ScrollBuf {
    ScrollBuf { buf: VecDeque::new() }
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
    writer.write(b"\x1b[s")?; // save pos
    writer.write(b"\x1b[15;1H")?; // go to start
    let mut line = 0;
    let mut idx = 0;
    for (i, &c) in self.buf.iter().enumerate().rev() {
      if c == b'\n' {
        line += 1;
      }
      if line > 30 {
        idx = i + 1;
        break;
      }
    }
    self.buf.drain(0..idx);
    let mut line = 0;
    for &c in &self.buf {
      if c == b'\n' {
        writer.write(format!("\x1b[{};1H", line + 15).as_bytes())?; // go to start
        line += 1;
      } else {
        writer.write(&[c])?;
      }
    }
    writer.write(b"\x1b[u")?; // restore pos
    writer.flush()?;
    Ok(())
  }
}

/// An appender which logs to standard out.
///
/// It supports output styling if standard out is a console buffer on Windows
/// or is a TTY on Unix.
#[derive(Debug)]
pub struct SkipConsoleAppender {
  skip:    usize,
  encoder: Box<dyn Encode>,
  buf:     Mutex<ScrollBuf>,
}

impl Append for SkipConsoleAppender {
  fn append(&self, record: &Record) -> anyhow::Result<()> {
    let mut buf = self.buf.lock().unwrap();
    self.encoder.encode(&mut AnsiWriter(&mut buf as &mut ScrollBuf), record)?;
    Ok(())
  }

  fn flush(&self) {}
}

impl SkipConsoleAppender {
  /// Creates a new `ConsoleAppender` builder.
  pub fn new<E: Encode>(skip: usize, encoder: E) -> SkipConsoleAppender {
    SkipConsoleAppender { skip, encoder: Box::new(encoder), buf: Mutex::new(ScrollBuf::new()) }
  }
}

pub fn skip_appender(skip: usize) -> Appender {
  Appender::builder()
    .build("stdout", Box::new(SkipConsoleAppender::new(skip, sc_common::make_pattern())))
}

pub fn setup() -> Result<(), io::Error> {
  let stdout = io::stdout();
  let mut w = stdout.lock();

  terminal::enable_raw_mode()?;
  execute!(io::stdout(), terminal::EnterAlternateScreen)?;

  ctrlc::set_handler(move || {
    execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
    println!("CONTROL C");
    std::process::exit(0);
  })
  .expect("Error setting Ctrl-C handler");

  w.write(b"\x1b[2J")?; // clear
  Ok(())
}

pub fn draw() -> Result<(), io::Error> {
  Ok(())
}

pub struct LineReader {
  buf:    ScrollBuf,
  prompt: &'static str,
}

impl LineReader {
  pub fn new(prompt: &'static str) -> Self {
    LineReader { buf: ScrollBuf::new(), prompt }
  }

  pub fn read_line(&mut self) -> Result<String, io::Error> {
    self.buf.write(self.prompt.as_bytes())?;
    self.buf.flush()?;

    let mut out = String::new();
    let mut reader = io::stdin();
    loop {
      let mut buf = [0; 1];
      reader.read(&mut buf)?;
      let c = buf[0];
      if c == b'\r' {
        self.buf.write(b"\n")?;
        self.buf.flush()?;
        break;
      }
      if c == b'q' {
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
        std::process::exit(0);
      }
      out.push(c as char);

      self.buf.write(&[c])?;
      self.buf.flush()?;
    }
    Ok(out)
  }
}
