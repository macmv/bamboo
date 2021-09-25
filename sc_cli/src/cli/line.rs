use super::ScrollBuf;
use crossterm::{execute, terminal};
use std::{
  io,
  io::{Read, Write},
};

pub struct SingleLineReader<'a> {
  buf:    &'a mut ScrollBuf,
  prompt: &'a str,
}

impl SingleLineReader<'_> {
  pub fn new<'a>(buf: &'a mut ScrollBuf, prompt: &'a str) -> SingleLineReader<'a> {
    SingleLineReader { buf, prompt }
  }

  pub fn read(&mut self) -> io::Result<String> {
    self.buf.write(self.prompt.as_bytes())?;
    self.buf.flush()?;

    let mut out = String::new();
    let mut reader = io::stdin();
    let mut in_escape = false;
    let mut escape = String::new();
    loop {
      let mut buf = [0; 1];
      reader.read(&mut buf)?;
      let c = buf[0];
      match c {
        b'\r' => {
          self.buf.write(b"\n")?;
          self.buf.flush()?;
          break;
        }
        b'\x03' => {
          // ctrl-c
          terminal::disable_raw_mode()?;
          execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
          std::process::exit(0);
        }
        b'\x7f' => {
          // backspace
          if !out.is_empty() {
            self.buf.back()?;
            out.pop();
          }
          continue;
        }
        b'\x1b' => {
          // escape (things like arrows keys)
          in_escape = true;
          continue;
        }
        _ => {}
      }
      if in_escape {
        escape.push(c as char);
        in_escape = self.parse_escape(&escape)?;
        if !in_escape {
          escape.clear();
        }
      } else {
        out.push(c as char);

        self.buf.write(&[c])?;
        self.buf.flush()?;
      }
    }
    Ok(out)
  }

  fn parse_escape(&self, code: &str) -> io::Result<bool> {
    let bytes = code.as_bytes();
    // incomplete
    if bytes.len() < 2 {
      return Ok(true);
    }
    if bytes[0] != b'[' {
      return Ok(false);
    }
    match bytes[1] {
      b'A' => io::stdout().write(b"a")?,       // up
      b'B' => io::stdout().write(b"b")?,       // down
      b'C' => io::stdout().write(b"\x1b[1C")?, // left
      b'D' => io::stdout().write(b"\x1b[1D")?, // right
      _ => 0,
    };
    io::stdout().flush()?;
    Ok(false)
  }
}
