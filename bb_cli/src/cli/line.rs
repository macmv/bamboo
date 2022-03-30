use super::ScrollBuf;
use crossterm::{execute, terminal};
use std::{
  io,
  io::{Read, Write},
};

pub struct SingleLineReader<'a> {
  buf:    &'a mut ScrollBuf,
  prompt: &'a str,
  // Char and byte index. This is where the cursor is on the screen. UTF8 screws this over, so if
  // UTF8 is going to be correctly implemented, this number should be where the cursor is, not a
  // byte index or anything.
  col:    u16,
  out:    String,
}

impl SingleLineReader<'_> {
  pub fn new<'a>(buf: &'a mut ScrollBuf, prompt: &'a str) -> SingleLineReader<'a> {
    SingleLineReader { buf, prompt, col: prompt.len() as u16, out: String::new() }
  }

  pub fn read(mut self) -> io::Result<String> {
    self.buf.write_all(self.prompt.as_bytes())?;
    self.buf.flush()?;
    let start_index = self.buf.buf().len();

    let mut reader = io::stdin();
    let mut in_escape = false;
    let mut escape = String::new();
    loop {
      let mut char_buf = [0; 4];
      let _ = reader.read(&mut char_buf)?;
      let c = match char::from_u32(u32::from_le_bytes(char_buf)) {
        Some(c) => c,
        None => continue,
      };
      // I couldn't be bothered to deal with unicode characters. The width is super
      // inconsistent, and messing with char vs byte indices for which column we are
      // in is not my problem. Someone else solve this.
      if !c.is_ascii() {
        continue;
      }
      match c {
        '\r' => {
          self.buf.write_all(b"\n")?;
          self.buf.flush()?;
          break;
        }
        '\x03' => {
          // ctrl-c
          terminal::disable_raw_mode()?;
          execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
          std::process::exit(0);
        }
        '\x7f' => {
          // backspace
          if !self.out.is_empty() {
            self.buf.back()?;
            self.out.pop();
            self.col -= 1;
          }
          continue;
        }
        '\x1b' => {
          // escape (things like arrows keys)
          in_escape = true;
          continue;
        }
        _ => {}
      }
      if in_escape {
        escape.push(c);
        in_escape = self.parse_escape(&escape)?;
        if !in_escape {
          escape.clear();
        }
      } else {
        if self.col == self.max_col() {
          self.out.push(c);
          self.buf.write_all(&[c as u8])?;
        } else {
          let min_col = self.min_col();
          self.out.insert((self.col - min_col) as usize, c);
          self.buf.buf().insert(start_index + (self.col - min_col) as usize, c as u8);
        }
        self.col += 1;

        self.buf.flush()?;

        if self.col != self.max_col() {
          write!(io::stdout(), "\x1b[{}D", self.out.len() as u16 - (self.col - self.min_col()))?;
          io::stdout().flush()?;
        }
      }
    }
    Ok(self.out)
  }

  fn parse_escape(&mut self, code: &str) -> io::Result<bool> {
    let bytes = code.as_bytes();
    // incomplete
    if bytes.len() < 2 {
      return Ok(true);
    }
    if bytes[0] != b'[' {
      return Ok(false);
    }
    match bytes[1] {
      b'A' => {} // up
      b'B' => {} // down
      b'C' => {
        // right
        if self.col >= self.max_col() {
          self.col = self.max_col();
        } else {
          self.col += 1;
          write!(io::stdout(), "\x1b[1C")?;
        }
      }
      b'D' => {
        // left
        if self.col <= self.min_col() {
          self.col = self.min_col();
        } else {
          self.col -= 1;
          write!(io::stdout(), "\x1b[1D")?;
        }
      }
      _ => {}
    };
    io::stdout().flush()?;
    Ok(false)
  }

  fn max_col(&self) -> u16 { self.prompt.len() as u16 + self.out.len() as u16 }
  fn min_col(&self) -> u16 { self.prompt.len() as u16 }
}
