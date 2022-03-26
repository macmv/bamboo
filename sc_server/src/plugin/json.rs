use crate::{block, player::Player};
use sc_common::{math::Pos, util::UUID};
use serde::{Deserialize, Serialize, Serializer};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, Serialize)]
pub struct JsonPlayer {
  pub username: String,
  pub uuid:     JsonUUID,
}

#[derive(Debug, Clone)]
pub struct JsonUUID {
  pub uuid: UUID,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonBlock {
  pub kind:  String,
  pub id:    u32,
  pub props: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JsonPos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

impl Serialize for JsonUUID {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.uuid.as_dashed_str())
  }
}

impl From<Arc<Player>> for JsonPlayer {
  fn from(p: Arc<Player>) -> Self {
    JsonPlayer { username: p.username().clone(), uuid: p.id().into() }
  }
}

impl From<UUID> for JsonUUID {
  fn from(uuid: UUID) -> Self { JsonUUID { uuid } }
}
impl From<Pos> for JsonPos {
  fn from(pos: Pos) -> Self { JsonPos { x: pos.x, y: pos.y, z: pos.z } }
}
impl From<JsonPos> for Pos {
  fn from(pos: JsonPos) -> Self { Pos { x: pos.x, y: pos.y, z: pos.z } }
}
impl From<block::Type> for JsonBlock {
  fn from(ty: block::Type) -> Self {
    JsonBlock { kind: ty.kind().to_str().into(), id: ty.id(), props: ty.props() }
  }
}
