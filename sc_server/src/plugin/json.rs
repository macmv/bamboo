use crate::player::Player;
use sc_common::{math::Pos, util::UUID};
use serde::{Serialize, Serializer};
use std::sync::Arc;

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
