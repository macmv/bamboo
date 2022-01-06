use super::Material;
use num_derive::{FromPrimitive, ToPrimitive};
use std::{error::Error, fmt, str::FromStr};

/// A single block type. This is different from a block kind, which is more
/// general. For example, there is one block kind for oak stairs. However, there
/// are 32 types for an oak stair, based on it's state (rotation, in this case).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type {
  pub(super) kind:  Kind,
  pub(super) state: u32,
}

impl Type {
  /// Returns the block kind that this state comes from.
  pub fn kind(&self) -> Kind { self.kind }
  /// Gets the block id of this type. This id is for the latest version of the
  /// game.
  pub fn id(&self) -> u32 { self.state }
}

#[derive(Debug)]
pub struct InvalidBlock(String);

impl fmt::Display for InvalidBlock {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid block name: {}", self.0)
  }
}

impl Error for InvalidBlock {}

include!(concat!(env!("OUT_DIR"), "/block/ty.rs"));

impl Kind {
  /// Returns the kind as an u32. This is used in the versioning arrays, and in
  /// plugin code, so that ints can be passed around instead of enums.
  pub fn id(self) -> u32 { num::ToPrimitive::to_u32(&self).unwrap() }
  /// Converts the given number to a block kind. If the number is invalid, this
  /// returns Kind::Air.
  pub fn from_u32(id: u32) -> Self { num::FromPrimitive::from_u32(id).unwrap_or(Kind::Air) }
}

/// A kind of bounding box. This is from prismarine data. It is not very
/// helpful, and will be replaced when I have a better data source.
#[derive(Debug)]
#[non_exhaustive]
pub enum BoundingBoxKind {
  Empty,
  Block,
}

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  /// The kind for this data.
  pub kind:         Kind,
  /// The name of this block. This is something like `grass_block`.
  pub name:         &'static str,
  /// The material used to make this block. This controls things like map color,
  /// sound, what tool breaks the block, etc. Prismarine doesn't have a very
  /// good material value, so this needs to be updated to more complete data.
  pub material:     Material,
  /// Amount of time it takes to break this block.
  pub hardness:     f32,
  /// How difficult this is to break with an explosion.
  pub resistance:   f32,
  /// A list of item ids this block can drop.
  pub drops:        &'static [u32],
  /// If this is true, then clients can (at least partially) see through this
  /// block.
  pub transparent:  bool,
  /// This is how much light this block removes. A value of 15 means it blocks
  /// all light, and a value of 0 means it blocks no light.
  pub filter_light: u8,
  /// The amount of light this block emits (0-15).
  pub emit_light:   u8,
  /// The kind of bounding box this block has.
  pub bounding_box: BoundingBoxKind,

  /// The latest version state id. This is the lowest possible state for this
  /// block. It is used to offset the state calculation for properties.
  pub state:     u32,
  /// All the properties on this block. These are stored so that it is easy to
  /// convert a single property on a block.
  props:         &'static [Prop],
  /// The default type. Each value is an index into that property.
  default_props: &'static [u32],
}

#[derive(Debug)]
pub struct Prop {
  name: &'static str,
  kind: PropKind,
}

#[derive(Debug)]
enum PropKind {
  Bool,
  Enum(&'static [&'static str]),
  Int { min: u32, max: u32 },
}

impl Data {
  /// Returns the default type for this kind. This is usually what should be
  /// placed down when the user right clicks on a block. Sometimes, for blocks
  /// like stairs or doors, the type that should be placed must be computed when
  /// they place the block, as things like their position/rotation affect which
  /// block gets placed.
  pub fn default_type(&self) -> Type {
    Type { kind: self.kind, state: self.resolve_state(self.default_props) }
  }

  fn resolve_state(&self, props: &[u32]) -> u32 {
    assert_eq!(self.props.len(), props.len());
    let mut id = 0;
    for (i, (p, idx)) in self.props.iter().zip(props).rev().enumerate() {
      id += idx;
      if i != props.len() - 1 {
        id *= p.len() as u32;
      }
    }
    self.state + id
  }
}

impl Prop {
  pub fn len(&self) -> u32 {
    match self.kind {
      PropKind::Bool => 2,
      PropKind::Enum(v) => v.len() as u32,
      PropKind::Int { min, max } => max - min + 1,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate() {
    // let kinds = generate_kinds();
    // Used to show debug output.
    // assert!(false);
  }
}
