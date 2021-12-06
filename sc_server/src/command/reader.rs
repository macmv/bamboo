use super::StringType;
use std::{
  error::Error,
  fmt,
  io::{self, BufRead, BufReader, Read},
  slice::Iter,
};

#[derive(Debug)]
pub enum ReadError {
  EOF,
  IO(io::Error),
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::EOF => write!(f, "Unexpected EOF while reading command"),
      Self::IO(e) => writeln!(f, "{} while reading command", e),
    }
  }
}

impl ReadError {
  pub fn pretty_print(&self, text: &str, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::EOF => writeln!(f, "Unexpected EOF while reading {}", text),
      Self::IO(e) => writeln!(f, "{} while reading {}", e, text),
    }
  }
}

impl Error for ReadError {}

impl From<io::Error> for ReadError {
  fn from(e: io::Error) -> ReadError {
    if e.kind() == io::ErrorKind::UnexpectedEof {
      ReadError::EOF
    } else {
      ReadError::IO(e)
    }
  }
}

pub struct CommandReader<'a> {
  buf: BufReader<StringReader<'a>>,
}

struct StringReader<'a> {
  iter: Iter<'a, u8>,
}

impl<'a> StringReader<'a> {
  pub fn new(data: &'a str) -> Self { Self { iter: data.as_bytes().iter() } }
}

impl<'a> Read for StringReader<'a> {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    for i in 0..buf.len() {
      if let Some(x) = self.iter.next() {
        buf[i] = *x;
      } else {
        return Ok(i);
      }
    }
    Ok(buf.len())
  }
}

impl<'a> CommandReader<'a> {
  pub fn new(text: &'a str) -> Self {
    CommandReader { buf: BufReader::new(StringReader::new(text)) }
  }
  /// Reads a string and space from the input stream. Depending on the
  /// StringType, this will parse a different amount of text:
  ///
  /// - `StringType::Word` will parse some text, and end at a space.
  /// - `StringType::Quotable` will parse some text, and end at a space. If the
  ///   first character is a double quote, then this will read to the next
  ///   double quote. Quotes can be escaped with a `\`.
  /// - `StringType::Greedy` will parse the rest of the string.
  pub fn word(&mut self, _ty: StringType) -> Result<String, ReadError> {
    let mut out = vec![];
    if self.buf.read_until(b' ', &mut out)? == 0 {
      return Err(ReadError::EOF);
    }
    if out.last() == Some(&b' ') {
      out.resize(out.len() - 1, 0);
    }
    // StringIter is iterating over a str, which is always valid utf8
    Ok(String::from_utf8(out).unwrap())
  }

  /// Reads a word, and parses it as an int.
  pub fn int(&mut self) -> Result<i32, ReadError> { Ok(5) }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn read_word() -> Result<(), ReadError> {
    // Single words
    let mut reader = CommandReader::new("i am big  space");

    assert_eq!("i", reader.word(StringType::Word)?);
    assert_eq!("am", reader.word(StringType::Word)?);
    assert_eq!("big", reader.word(StringType::Word)?);
    assert_eq!("", reader.word(StringType::Word)?);
    assert_eq!("space", reader.word(StringType::Word)?);
    matches!(reader.word(StringType::Word).unwrap_err(), ReadError::EOF);

    // UTF8 tests
    let mut reader = CommandReader::new("weird 🍔∈🌏 characters");

    assert_eq!("weird", reader.word(StringType::Word)?);
    assert_eq!("🍔∈🌏", reader.word(StringType::Word)?);
    assert_eq!("characters", reader.word(StringType::Word)?);
    matches!(reader.word(StringType::Word).unwrap_err(), ReadError::EOF);

    Ok(())
  }
}
