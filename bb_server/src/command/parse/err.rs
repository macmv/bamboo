use super::{super::ErrorFormat, Parser, Span};
use bb_common::util::{chat::Color, Chat};
use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
  kind: ErrorKind,
  pos:  Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChildError {
  /// Used for a general error. The parser's desc will will be used in the
  /// `expected` message.
  Invalid(Parser),
  // Used for a specific error. This string will be directly used in the
  // `expected` message.
  Expected(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
  /// Used when no children of the node matched
  NoChildren(Vec<ChildError>),
  /// Used when a literal does not match, or a parser fails. If more detail is
  /// needed, use [`expected`](super::Word::expected).
  Invalid,
  /// Used when there are trailing characters after the command
  Trailing,
  /// Used when the string ends early.
  EOF,
  /// Value is what was actually expected.
  Expected(String),
  /// Used when a value is out of range
  Range(f64, Option<f64>, Option<f64>),
}

pub type Result<T> = std::result::Result<T, ParseError>;

impl ParseError {
  /// Creates a new pos error. This error will cover all the characters in
  /// `pos`, and have the error message of the given `kind`.
  pub fn new(pos: Span, kind: ErrorKind) -> Self { ParseError { pos, kind } }

  /// Returns the error kind for this error.
  pub fn kind(&self) -> &ErrorKind { &self.kind }
  /// Returns the position of this this error.
  pub fn pos(&self) -> Span { self.pos }

  /// Generates a chat message from the error. This should be sent directly to
  /// the client without any additional formatting.
  pub fn to_chat(&self, text: &str, format: ErrorFormat) -> Chat {
    let mut out = Chat::new("");
    let prefix = "Invalid command: ";
    match format {
      ErrorFormat::Minecraft => {
        out.add(prefix).color(Color::Red);
        if self.pos.start == text.len() {
          out.add(format!("{text} ")).color(Color::White);
          out.add(" ").color(Color::Red).underlined();
        } else {
          out.add(&text[..self.pos.start]).color(Color::White);
          out.add(&text[self.pos.start..self.pos.end]).color(Color::Red).underlined();
          out.add(&text[self.pos.end..]).color(Color::White);
        }
        out.add(format!("\n  -> {}", self.kind)).color(Color::White);
      }
      ErrorFormat::Monospace => {
        out.add(format!("{prefix}\n")).color(Color::Red);
        out.add(format!("  {text}\n")).color(Color::White);
        if self.pos.start == text.len() {
          out.add(format!("  {}^", " ".repeat(text.len() + 1))).color(Color::Red);
        } else {
          out
            .add(format!(
              "  {}{}",
              " ".repeat(self.pos.start),
              "^".repeat(self.pos.end - self.pos.start)
            ))
            .color(Color::Red);
        }
        out.add(format!(" {}", self.kind)).color(Color::Red);
      }
    }

    out
  }
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "error at {}:{}: {}", self.pos.start, self.pos.end, self.kind)
  }
}
impl fmt::Display for ChildError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Invalid(p) => write!(f, "{}", p.desc()),
      Self::Expected(s) => write!(f, "`{s}`"),
    }
  }
}
impl fmt::Display for ErrorKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoChildren(errors) => {
        if errors.is_empty() {
          // No errors means print another error about no errors
          write!(f, "no parsers in no children error (should never happen)")
        } else if errors.len() == 1 {
          // A single parser failed, also invalid
          write!(f, "only one parser failed, not valid")
        } else {
          // Write all of the children in a row
          write!(f, "expected ")?;
          for (i, e) in errors.iter().enumerate() {
            if i == errors.len() - 1 {
              write!(f, "or {e}")?;
            } else if i == errors.len() - 2 {
              write!(f, "{e} ")?;
            } else {
              write!(f, "{e}, ")?;
            }
          }
          Ok(())
        }
      }
      // Not really used. Parser::desc overrides these
      Self::Invalid => write!(f, "invalid"),
      Self::Trailing => write!(f, "trailing characters"),
      Self::EOF => write!(f, "command ended early"),
      Self::Expected(expected) => {
        write!(f, "expected {expected}")
      }
      Self::Range(v, min, max) => {
        if let (Some(min), Some(max)) = (min, max) {
          write!(f, "{v} is out of range {min}..{max}")
        } else if let Some(min) = min {
          write!(f, "{v} is less than min {min}")
        } else if let Some(max) = max {
          write!(f, "{v} is greater than max {max}")
        } else {
          write!(f, "{v} is out of range none (should never happen)")
        }
      }
    }
  }
}

impl Error for ParseError {}
