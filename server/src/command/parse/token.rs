use super::{ParseError, Result};
use std::{iter::Peekable, str::Chars};

pub struct Tokenizer<'a> {
  text: Peekable<Chars<'a>>,
  pos:  usize,
}

/// A region of text in a commmand. The start is inclusive, and the end is
/// exclusive.
pub struct Span {
  pub start: usize,
  pub end:   usize,
}

pub struct Word {
  text: String,
  pos:  Span,
}

impl<'a> Tokenizer<'a> {
  pub fn new(text: &'a str) -> Self {
    Tokenizer { text: text.chars().peekable(), pos: 0 }
  }

  /// Reads a single char.
  fn char(&mut self) -> Option<char> {
    self.pos += 1;
    self.text.next()
  }

  /// Reads a single word. This can be terminated by a single space, or by a
  /// non-alphabet character.
  pub fn read_word(&mut self) -> Result<Word> {
    let mut text = String::new();
    let start = self.pos;
    if let Some(c) = self.text.peek() {
      if !c.is_ascii_alphabetic() {
        return Err(ParseError::InvalidText(c.to_string(), "a letter".into()));
      }
    } else {
      return Err(ParseError::EOF);
    }
    while let Some(c) = self.text.peek() {
      if !c.is_ascii_alphabetic() {
        // Skip whitespace
        if c.is_whitespace() {
          self.pos += 1;
        }
        break;
      }
      text.push(self.text.next().unwrap());
      self.pos += 1;
    }
    Ok(Word { text, pos: Span { start, end: self.pos - 1 } })
  }

  /// Reads a single word. This must be terminated by a space. Any non-alphabet
  /// characters are considered invalid.
  pub fn read_spaced_word(&mut self) -> Result<Word> {
    let mut text = String::new();
    let start = self.pos;
    if let Some(c) = self.text.peek() {
      if !c.is_ascii_alphabetic() {
        return Err(ParseError::InvalidText(c.to_string(), "a letter".into()));
      }
    } else {
      return Err(ParseError::EOF);
    }
    while let Some(c) = self.text.peek() {
      if !c.is_ascii_alphabetic() {
        return Err(ParseError::InvalidText(c.to_string(), "a letter".into()));
      }
      text.push(self.text.next().unwrap());
      self.pos += 1;
    }
    Ok(Word { text, pos: Span { start, end: self.pos - 1 } })
  }
}

impl PartialEq<&str> for Word {
  fn eq(&self, text: &&str) -> bool {
    self.text == *text
  }
}

impl Word {
  pub fn invalid<R: Into<String>>(&self, reason: R) -> ParseError {
    ParseError::InvalidText(self.text.clone(), reason.into())
  }
}
