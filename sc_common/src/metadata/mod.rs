use crate::{
  math::Pos,
  util::{Chat, Face, Item, UUID},
  version::ProtocolVersion,
};
use sc_macros::Transfer;
use std::collections::HashMap;

#[derive(Transfer)]
pub struct Metadata {
  // A sparse map of indices to serialized fields.
  fields: HashMap<u8, Field>,
}

#[derive(Transfer)]
pub enum Pose {
  #[id = 0]
  Standing,
  #[id = 1]
  FallFlying,
  #[id = 2]
  Sleeping,
  #[id = 3]
  Swimming,
  #[id = 4]
  SpinAttack,
  #[id = 5]
  Sneaking,
  #[id = 6]
  Dying,
}

impl Default for Pose {
  fn default() -> Pose { Pose::Standing }
}

/// The types for each metadata field. Updated to the latest version of the
/// game.
#[derive(Transfer)]
pub enum Field {
  // Only valid on 1.8
  #[id = 0]
  Short(i16),
  #[id = 1]
  Int(i32),

  // Valid for all versions
  #[id = 2]
  Byte(u8),
  #[id = 3]
  Float(f32),
  #[id = 4]
  String(String),
  #[id = 5]
  Item(Item),
  #[id = 6]
  Position(Pos),
  #[id = 7]
  Rotation(f32, f32, f32), // Rotation on x, y, z

  // Valid in 1.9+
  #[id = 8]
  Varint(i32),
  /// JSON encoded chat message
  #[id = 9]
  Chat(String),
  #[id = 10]
  Bool(bool),
  #[id = 11]
  OptPosition(Option<Pos>),
  #[id = 12]
  Direction(Face),
  #[id = 13]
  OptUUID(Option<UUID>),
  #[id = 14]
  BlockID(i32),

  // Valid for 1.12+
  #[id = 15]
  NBT(Vec<u8>), // TODO: Implement NBT

  // Valid for 1.13+
  /// JSON encoded chat message
  #[id = 16]
  OptChat(Option<String>),
  #[id = 17]
  Particle(Vec<u8>), // TODO: Implement particle data

  // Valid for 1.14+
  #[id = 18]
  VillagerData(i32, i32, i32),
  #[id = 19]
  OptVarint(Option<i32>),
  #[id = 20]
  Pose(Pose),
}

impl Metadata {
  pub fn new() -> Self { Metadata { fields: HashMap::new() } }
}
