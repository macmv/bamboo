use super::{Arg, Parser};

#[derive(Debug)]
pub enum ParseError {
  InvalidLiteral(String),
}

impl Parser {
  pub fn parse(&self, text: &str) -> Result<(Arg, usize), ParseError> {
    Ok((Arg::Int(5), 1))
  }
}
