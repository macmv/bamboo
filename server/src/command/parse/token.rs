use super::{ParseError, Result};
use std::{iter::Peekable, str::Chars};

pub struct Tokenizer<'a> {
  text:     &'a str,
  chars:    Peekable<Chars<'a>>,
  char_pos: usize,
  byte_pos: usize,
}

/// A region of text in a commmand. The start is inclusive, and the end is
/// exclusive. Both indices are in chars, not bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
  pub start: usize,
  pub end:   usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
  text: String,
  pos:  Span,
}

impl<'a> Tokenizer<'a> {
  pub fn new(text: &'a str) -> Self {
    Tokenizer { text, chars: text.chars().peekable(), char_pos: 0, byte_pos: 0 }
  }

  fn peek_char(&mut self) -> Option<char> {
    self.chars.peek().map(|v| *v)
  }

  fn next_char(&mut self) -> Option<char> {
    self.chars.next().map(|v| {
      self.char_pos += 1;
      self.byte_pos += v.len_utf8();
      v
    })
  }

  /// Reads a single word. This can be terminated by a single space, or by a
  /// non-alphabet character.
  pub fn read_word(&mut self) -> Result<Word> {
    let mut text = String::new();
    let start = self.char_pos;
    let mut end = 0;
    if let Some(c) = self.peek_char() {
      if !c.is_ascii_alphabetic() {
        return Err(ParseError::InvalidText(c.to_string(), "a letter".into()));
      }
    } else {
      return Err(ParseError::EOF);
    }
    while let Some(c) = self.peek_char() {
      if !c.is_ascii_alphabetic() {
        // Skip whitespace
        end = self.char_pos;
        if c.is_whitespace() {
          self.next_char().unwrap();
        }
        break;
      }
      text.push(self.next_char().unwrap());
    }
    if end == 0 {
      Ok(Word { text, pos: Span { start, end: self.char_pos } })
    } else {
      Ok(Word { text, pos: Span { start, end } })
    }
  }

  /// Reads a single word. This must be terminated by a space. Any non-alphabet
  /// characters are considered invalid.
  pub fn read_spaced_word(&mut self) -> Result<Word> {
    let mut text = String::new();
    let start = self.char_pos;
    let mut end = 0;
    if let Some(c) = self.peek_char() {
      if !c.is_ascii_alphabetic() {
        return Err(ParseError::InvalidText(c.to_string(), "a letter".into()));
      }
    } else {
      return Err(ParseError::EOF);
    }
    while let Some(c) = self.peek_char() {
      if c.is_whitespace() {
        end = self.char_pos;
        self.next_char().unwrap();
        break;
      }
      if !c.is_ascii_alphabetic() {
        return Err(ParseError::InvalidText(c.to_string(), "a letter".into()));
      }
      text.push(self.next_char().unwrap());
    }
    if end == 0 {
      // Happens when we reach the end of the string
      Ok(Word { text, pos: Span { start, end: self.char_pos } })
    } else {
      Ok(Word { text, pos: Span { start, end } })
    }
  }

  /// Checks for trailing characters. If there are any unread characters, this
  /// will return an error.
  pub fn check_trailing(&mut self) -> Result<()> {
    if let Some(c) = self.peek_char() {
      Err(ParseError::Trailing(self.text[self.byte_pos..].to_string()))
    } else {
      Ok(())
    }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_types() {
    let mut tok = Tokenizer::new("i am spaced");
    assert_eq!(
      tok.read_spaced_word(),
      Ok(Word { text: "i".into(), pos: Span { start: 0, end: 1 } })
    );
    assert!(tok.check_trailing().is_err());
    assert_eq!(
      tok.read_spaced_word(),
      Ok(Word { text: "am".into(), pos: Span { start: 2, end: 4 } })
    );
    assert!(tok.check_trailing().is_err());
    assert_eq!(
      tok.read_spaced_word(),
      Ok(Word { text: "spaced".into(), pos: Span { start: 5, end: 11 } })
    );
    assert!(tok.check_trailing().is_ok());
    assert_eq!(tok.read_spaced_word(), Err(ParseError::EOF));
    assert!(tok.check_trailing().is_ok());
    assert_eq!(tok.read_spaced_word(), Err(ParseError::EOF));
    assert!(tok.check_trailing().is_ok());
  }
}
