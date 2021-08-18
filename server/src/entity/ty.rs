use num_derive::{FromPrimitive, ToPrimitive};

// Creates the Type enum, and the generate_entities function.
data::generate_entities!();

/// Any data specific to an entity.
#[derive(Debug)]
pub struct Data {
  display_name: &'static str,
  width:        f32,
  height:       f32,
}

impl Data {
  pub fn display_name(&self) -> &str {
    &self.display_name
  }
}

impl Type {
  /// Returns the kind as a u32. Should only be used to index into the
  /// converter's internal table of block kinds.
  pub fn to_u32(self) -> u32 {
    num::ToPrimitive::to_u32(&self).unwrap()
  }
  /// Returns the item with the given id. If the id is invalid, this returns
  /// `Type::Air`.
  pub fn from_u32(v: u32) -> Type {
    num::FromPrimitive::from_u32(v).unwrap_or(Type::None)
  }
}
