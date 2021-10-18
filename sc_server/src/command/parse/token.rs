use super::{ErrorKind, ParseError, Result};
use std::{iter::Peekable, ops::Deref, str::Chars};

#[derive(Clone)]
pub struct Tokenizer<'a> {
  text:  &'a str,
  chars: Peekable<Chars<'a>>,
  // Pos in bytes
  pos:   usize,
}

/// A region of text in a commmand. The start is inclusive, and the end is
/// exclusive. Both indices are in bytes, not chars. It is considered invalid
/// state for a Span's start or end to not be on a utf8 boundry.
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
    Tokenizer { text, chars: text.chars().peekable(), pos: 0 }
  }

  /// Returns the current index into the command of this tokenizer. This will
  /// always be at the start of a valid utf8 character.
  pub fn pos(&self) -> usize {
    self.pos
  }

  fn peek_char(&mut self) -> Option<char> {
    self.chars.peek().map(|v| *v)
  }

  fn next_char(&mut self) -> Option<char> {
    self.chars.next().map(|v| {
      self.pos += v.len_utf8();
      v
    })
  }

  /// Reads a single word. This can be terminated by a single space, or by a
  /// non-alphabet character.
  pub fn read_word(&mut self) -> Result<Word> {
    let mut text = String::new();
    let start = self.pos;
    let mut end = 0;
    if let Some(c) = self.peek_char() {
      if !c.is_ascii_alphabetic() {
        return Err(ParseError::new(Span::single(start), ErrorKind::Expected("a letter".into())));
      }
    } else {
      return Err(ParseError::new(Span::single(self.pos), ErrorKind::EOF));
    }
    while let Some(c) = self.peek_char() {
      if !c.is_ascii_alphabetic() {
        // Skip whitespace
        end = self.pos;
        if c.is_whitespace() {
          self.next_char().unwrap();
        }
        break;
      }
      text.push(self.next_char().unwrap());
    }
    if end == 0 {
      Ok(Word { text, pos: Span { start, end: self.pos } })
    } else {
      Ok(Word { text, pos: Span { start, end } })
    }
  }

  /// Reads a single word. This must be terminated by a space. Any non-alphabet
  /// characters are considered invalid.
  pub fn read_spaced_word(&mut self) -> Result<Word> {
    let mut text = String::new();
    let start = self.pos;
    let mut end = 0;
    if self.peek_char().is_none() {
      // We want EOF errors to be one letter past the last one, to make invalid last
      // characters and EOFs underline different things.
      return Err(ParseError::new(Span::single(self.pos), ErrorKind::EOF));
    }
    let mut valid = true;
    while let Some(c) = self.peek_char() {
      if c.is_whitespace() {
        end = self.pos;
        self.next_char().unwrap();
        break;
      }
      if !c.is_ascii_alphabetic() {
        valid = false;
      }
      text.push(self.next_char().unwrap());
    }
    let pos = if end == 0 {
      // Happens when we reach the end of the string
      Span::new(start, self.pos)
    } else {
      Span::new(start, end)
    };
    if valid {
      Ok(Word { pos, text })
    } else {
      Err(ParseError::new(pos, ErrorKind::Expected("a letter".into())))
    }
  }
  /// Reads a single non-alphabetic word. This must be terminated by a space.
  /// There are no restrictions on what letters are valid in this region of
  /// text.
  pub fn read_spaced_text(&mut self) -> Result<Word> {
    let mut text = String::new();
    let start = self.pos;
    let mut end = 0;
    if self.peek_char().is_none() {
      // We want EOF errors to be one letter past the last one, to make invalid last
      // characters and EOFs underline different things.
      return Err(ParseError::new(Span::single(self.pos), ErrorKind::EOF));
    }
    while let Some(c) = self.peek_char() {
      if c.is_whitespace() {
        end = self.pos;
        self.next_char().unwrap();
        break;
      }
      text.push(self.next_char().unwrap());
    }
    let pos = if end == 0 {
      // Happens when we reach the end of the string
      Span::new(start, self.pos)
    } else {
      Span::new(start, end)
    };
    Ok(Word { pos, text })
  }

  /// Checks for trailing characters. If there are any unread characters, this
  /// will return an error.
  pub fn check_trailing(&mut self) -> Result<()> {
    if let Some(_) = self.peek_char() {
      let s = self.text[self.pos..].to_string();
      Err(ParseError::new(Span::new(self.pos, self.pos + s.len()), ErrorKind::Expected(s)))
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
impl Deref for Word {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    &self.text
  }
}
impl From<Word> for String {
  fn from(w: Word) -> String {
    w.text
  }
}

impl Word {
  /// Returns an error with the span covering this word. Use this when you get
  /// a complex error. For things like a keyword mismatch, prefer
  /// [`invalid`](Self::invalid). This is mostly useful for a rotation being out
  /// of range, or similar.
  pub fn expected<R: Into<String>>(&self, reason: R) -> ParseError {
    ParseError::new(self.pos, ErrorKind::Expected(reason.into()))
  }

  /// Returns an invalid error. This will use the `Parser::desc` function later
  /// to produce an error spanned over this word.
  pub fn invalid(&self) -> ParseError {
    ParseError::new(self.pos, ErrorKind::Invalid)
  }

  /// Returns the position of this word.
  pub fn pos(&self) -> Span {
    self.pos
  }

  /// Upates the internal text of this word, without changing the span.
  pub fn set_text(&mut self, new_text: String) {
    self.text = new_text;
  }
}

impl Span {
  /// Creates a new span that wraps the text between the two indices. The
  /// `start` is inclusive, and the `end` is exlusive.
  pub fn new(start: usize, end: usize) -> Self {
    Span { start, end }
  }
  /// Creates a span that wraps a single char at index `char_index`.
  pub fn single(char_index: usize) -> Self {
    Span { start: char_index, end: char_index + 1 }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tokenizer() {
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
    assert_eq!(tok.read_spaced_word().unwrap_err().kind(), &ErrorKind::EOF);
    assert!(tok.check_trailing().is_ok());
    assert_eq!(tok.read_spaced_word().unwrap_err().kind(), &ErrorKind::EOF);
    assert!(tok.check_trailing().is_ok());
  }
}
