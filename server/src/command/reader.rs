use super::StringType;
use std::{
  error::Error,
  fmt,
  io::{self, BufReader, Read},
  slice::Iter,
};

#[derive(Debug, PartialEq)]
pub enum ReadError {
  EOF(String),
  Unexpected(String, usize, char),
  Invalid(String, usize, char),
}

impl fmt::Display for ReadError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::EOF(text) => writeln!(f, "Unexpected EOF while reading {}", text),
      Self::Unexpected(text, index, c) => {
        let s = format!("Unexpected '{}' while reading {}", c, text);
        let index = s.len() - text.len() + index;
        writeln!(f, "{}", s)?;
        writeln!(f, "{}^ here", " ".repeat(index - 1))
      }
      Self::Invalid(text, index, c) => {
        let s = format!("Invalid '{}' while reading {}", c, text);
        let index = s.len() - text.len() + index;
        writeln!(f, "{}", s);
        writeln!(f, "{}^ here", " ".repeat(index - 1))
      }
    }
  }
}

impl Error for ReadError {}

pub struct CommandReader<'a> {
  buf: BufReader<StringReader<'a>>,
}

struct StringReader<'a> {
  iter: Iter<'a, u8>,
}

impl<'a> StringReader<'a> {
  pub fn new(data: &'a str) -> Self {
    Self { iter: data.as_bytes().iter() }
  }
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
  pub fn word(&mut self, ty: StringType) -> Result<String, ReadError> {
    Ok("".into())
  }

  /// Reads a word, and parses it as an int.
  pub fn int(&mut self) -> Result<i32, ReadError> {
    Ok(5)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn read_word() -> Result<(), ReadError> {
    // Single words
    let mut reader = CommandReader::new("hello world i am big  space");

    assert_eq!("hello", reader.word(StringType::Word)?);
    assert_eq!("world", reader.word(StringType::Word)?);
    assert_eq!("i", reader.word(StringType::Word)?);
    assert_eq!("am", reader.word(StringType::Word)?);
    assert_eq!("big", reader.word(StringType::Word)?);
    assert_eq!("", reader.word(StringType::Word)?);
    assert_eq!("space", reader.word(StringType::Word)?);
    matches!(reader.word(StringType::Word).unwrap_err(), EOF);

    Ok(())
  }
}
