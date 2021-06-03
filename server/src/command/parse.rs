use super::{Arg, Parser};
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ParseError {
  /// Used when a literal does not match
  InvalidLiteral(String),
  /// Used when no children of the node matched
  NoChildren(Vec<ParseError>),
  /// Used when there are trailing characters after the command
  Trailing(String),
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
    }
  }
}

impl Error for ParseError {}

impl Parser {
  pub fn parse(&self, text: &str) -> Result<(Arg, usize), ParseError> {
    Ok((Arg::Int(5), 1))
  }
}
