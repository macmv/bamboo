use super::{Metadata, Type};
use std::{error::Error, fmt};

#[derive(Debug)]
pub struct IndexError(u8);

impl fmt::Display for IndexError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid index (max is 254): {}", self.0)
  }
}

impl Error for IndexError {}

impl Metadata {
  pub fn set_byte(&mut self, index: u8, value: u8) -> Result<(), IndexError> {
    self.set_field(index, vec![value])
  }

  fn set_field(&mut self, index: u8, value: Vec<u8>) -> Result<(), IndexError> {
    if index > 254 {
      return Err(IndexError(index));
    }
    self.fields.insert(index, value);
    Ok(())
  }
}
