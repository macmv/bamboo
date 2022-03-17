use crate::dl;
use serde::Deserialize;
use std::{fs, io, path::Path};

mod cross;
mod gen;
mod meta;

pub fn generate(out_dir: &Path) -> io::Result<()> {
  fs::create_dir_all(out_dir.join("entity"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let def: EntityDef = dl::get("entities", ver);
      (ver, def)
    })
    .collect();
  gen::generate(versions, &out_dir.join("entity"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityDef {
  entities: Vec<Option<Entity>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Entity {
  /// The id of the entity.
  id:    u32,
  /// The name of the entity.
  name:  String,
  /// The full class of this entity.
  class: String,

  category:       String,
  width:          f32,
  height:         f32,
  tracking_range: u32,

  metadata: Vec<MetadataField>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataField {
  /// The index of this metadata field.
  id:   u32,
  /// The name of this field. This is how cross-versioning works. We use the
  /// yarn mappings here, and convert MCP mappings into yarn mappings.
  name: String,
  /// The kind of metadata.
  ty:   MetadataType,
}

/// An entity metadata type. Note that the documentation for this type is for
/// 1.18.2. Older versions will have different serializing/deserializing rules.
#[derive(Debug, Clone, Deserialize)]
pub enum MetadataType {
  /// A single byte.
  Byte,
  /// A varint (same as protocol).
  VarInt,
  /// A short. Only present on 1.8-1.12.
  Short,
  /// A 4 byte floating point number
  Float,
  /// A varint prefixed string
  String,
  /// A string, which is JSON encoded chat data.
  Chat,
  /// A boolean. If true, this is followed by a Chat field.
  OptChat,
  /// An item stack. Same as protocol.
  Item,
  /// A single byte.
  Bool,
  /// 3 floats for X, Y, then Z.
  Rotation,
  /// A position encoded as a long.
  Position,
  /// A boolean. If true, this is followed by a Position.
  OptPosition,
  /// A VarInt. This will be from 0-5 (inclusive), which maps to a direction
  /// like so:
  /// - 0: Down
  /// - 1: Up
  /// - 2: North
  /// - 3: South
  /// - 4: West
  /// - 5: East
  Direction,
  /// A boolean. If true, then a 16 byte UUID follows.
  OptUUID,
  /// A varint, which should be parsed as a block ID.
  BlockID,
  /// An NBT tag. This is not length prefixed. The entire tag must be parsed to
  /// find the end of this field.
  NBT,
  /// A VarInt for the particle ID, followed by some data. The data following
  /// must be infered from the particle ID.
  Particle,
  /// 3 VarInts: villager type, villager profession, and villager level.
  VillagerData,
  /// A boolean. If true, a VarInt follows.
  OptVarInt,
  /// A VarInt, from 0-7 (inclusive). The numbers map to these poses:
  /// - 0: Standing
  /// - 1: Fall flying
  /// - 2: Sleeping
  /// - 3: Swiming
  /// - 4: Spin attack
  /// - 5: Sneaking
  /// - 6: Long jumping
  /// - 7: Dying
  Pose,

  /// TODO: Figure out what this is!
  FireworkData,
}
