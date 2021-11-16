use crate::dl;
use serde::Deserialize;
use std::{io, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct BlockDef {
  blocks: Vec<Block>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Block {
  /// The id of the block.
  id:               u32,
  /// The name id, used everywhere imporant.
  name:             String,
  /// The name used in lang files.
  unlocalized_name: String,
  /// The full class of the block.
  class:            String,

  /// The enum name of the material.
  material:  String,
  /// The enum name of the map color. Defaults to the map color of the material.
  map_color: String,

  /// The explosion resistance of the block.
  resistance: f32,
  /// The time it takes to mine this block.
  hardness:   f32,

  /// The amount of light this block emits. Will be a number from 0 to 15. This
  /// is zero for most blocks, but will be set for things like torches.
  luminance:    u8,
  /// The slipperiness factor. If set to 0, then this is a normal block.
  /// Otherwise, this is some factor used for ice. Currently, it is always 0.98
  /// for ice, and 0 for everything else.
  slipperiness: f32,

  /// Set when this block doesn't have a hitbox.
  no_collision: bool,
  /// Enum variant of the sound this block makes when walked on.
  sound_type:   String,

  /// A list of items this block drops.
  drops: Vec<String>,

  /// A list of all the properties on this block. If the states are empty, there
  /// is a single valid state for this block, which has no properties. See the
  /// [`State`] docs for more.
  properties: Vec<Prop>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Prop {
  /// The name of this property. This will be something like `rotation` or
  /// `waterlogged`, for example.
  name: String,

  /// The possible values of this state.
  kind: PropKind,
}

#[derive(Debug, Clone, Deserialize)]
pub enum PropKind {
  /// A boolean property. This can either be `true` or `false`.
  Bool,
  /// An enum property. This can be any of the given values.
  Enum(Vec<String>),
  /// A number property. This can be anything from `min..=max`, where `max` is
  /// the inclusive end of the range. The start is normally zero, but can
  /// sometimes be one.
  Int { min: u32, max: u32 },
}

pub fn generate(out_dir: &Path) -> io::Result<()> {
  for &ver in crate::VERSIONS {
    let def: BlockDef = dl::get("blocks", ver);
  }
  Ok(())
}
