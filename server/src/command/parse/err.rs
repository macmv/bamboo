use std::{error::Error, fmt};

#[derive(Debug, PartialEq)]
pub enum ParseError {
  /// Used when a literal does not match
  InvalidLiteral(String),
  /// Used when no children of the node matched
  NoChildren(Vec<ParseError>),
  /// Used when there are trailing characters after the command
  Trailing(String),
  /// Used when the string ends early.
  EOF,
  /// Used whenever a field does not match the given text.
  /// Values are the text, and the expected value.
  InvalidText(String, String),
  /// Used when a value is out of range
  Range(f64, Option<f64>, Option<f64>),
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidLiteral(v) => write!(f, "invalid literal: {}", v),
      Self::NoChildren(errors) => {
        if errors.is_empty() {
          // No errors means print another error about no errors
          write!(f, "no errors in no children error (should never happen)")
        } else if errors.len() == 1 {
          // A single error should just be printed as that error
          write!(f, "{}", errors[0])
        } else {
          // Write all of the children in a row
          writeln!(f, "no children matched: [")?;
          for e in errors {
            write!(f, "  {}", e)?;
          }
          write!(f, "]")
        }
      }
      Self::Trailing(v) => write!(f, "trailing characters: {}", v),
      Self::EOF => write!(f, "string ended early"),
      Self::InvalidText(text, expected) => {
        write!(f, "invalid text: {}. expected {}", text, expected)
      }
      Self::Range(v, min, max) => {
        if let (Some(min), Some(max)) = (min, max) {
          write!(f, "{} is out of range {}..{}", v, min, max)
        } else if let Some(min) = min {
          write!(f, "{} is less than min {}", v, min)
        } else if let Some(max) = max {
          write!(f, "{} is greater than max {}", v, max)
        } else {
          write!(f, "{} is out of range none (should never happen)", v)
        }
      }
    }
  }
}

impl Error for ParseError {}
