use std::{
  io::{self, BufReader, Read},
  slice::Iter,
};

pub enum ReadError {
  EOF,
  Unexpected(usize, char),
  Invalid(usize, char),
}

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
  pub fn read_str(&mut self) -> Result<i32, ReadError> {
    Ok(5)
  }

  /// Reads a word, and parses it as an int.
  pub fn read_int(&mut self) -> Result<i32, ReadError> {
    Ok(5)
  }
}
