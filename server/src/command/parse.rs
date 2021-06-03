use super::{Arg, Parser};

#[derive(Debug)]
pub enum ParseError {
  /// Used when a literal does not match
  InvalidLiteral(String),
  /// Used when no children of the node matched
  NoChildren(Vec<ParseError>),
}

impl Parser {
  pub fn parse(&self, text: &str) -> Result<(Arg, usize), ParseError> {
    Ok((Arg::Int(5), 1))
  }
}
