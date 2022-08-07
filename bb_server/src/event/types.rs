use super::json::*;
use crate::{block, math::Vec3, player::Player, world::MultiChunk};
use bb_common::{
  math::{ChunkPos, Pos},
  net::sb::ClickWindow,
};
use parking_lot::Mutex;
use std::sync::Arc;

/// A message going from the plugin to the server.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum PluginMessage {
  Event {
    #[serde(flatten)]
    event: PluginEvent,
  },
  Request {
    reply_id: u32,
    #[serde(flatten)]
    request:  PluginRequest,
  },
  Reply {
    reply_id: u32,
    #[serde(flatten)]
    reply:    PluginReply,
  },
}

/// A message to the server. The server cannot reply to this message. Once sent,
/// the plugin should forget about it.
#[non_exhaustive]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PluginEvent {
  Register { ty: String },
  Ready,

  SendChat { text: String },
}
/// A request from the plugin to the server. The id to reply with is stored in
/// [PluginMessage]. Once sent, the plugin should expect a reply from the server
/// soon. See also [ServerReply].
#[non_exhaustive]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PluginRequest {
  GetBlock { pos: JsonPos },
}
/// A response to a request from the server. See also [ServerRequest].
#[non_exhaustive]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PluginReply {
  Cancel { allow: bool },
}

/// Any message going from the server to the plugin.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind")]
pub enum ServerMessage {
  Event {
    #[serde(serialize_with = "to_json_ty::<_, JsonPlayer, _>")]
    player: Arc<Player>,
    #[serde(flatten)]
    event:  ServerEvent,
  },
  GlobalEvent {
    #[serde(flatten)]
    event: GlobalServerEvent,
  },
  Request {
    reply_id: u32,
    #[serde(serialize_with = "to_json_ty::<_, JsonPlayer, _>")]
    player:   Arc<Player>,
    #[serde(flatten)]
    request:  ServerRequest,
  },
  Reply {
    reply_id: u32,
    #[serde(flatten)]
    reply:    ServerReply,
  },
}

/// An event from the server to the plugin. There is also a player listed with
/// this event.
#[non_exhaustive]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
  Chat { text: String },
  PlayerJoin,
  PlayerLeave,
}
/// An event from the server to the plugin. This is very similar to
/// [ServerEvent], but there is no player specified with this event.
#[non_exhaustive]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum GlobalServerEvent {
  Tick,
  /// The plugin should fill the given chunk with the terrain for the given
  /// generator. MultiChunk is an arc, so this should be mutated.
  GenerateChunk {
    generator: String,
    #[serde(skip)]
    chunk:     Arc<Mutex<MultiChunk>>,
    #[serde(skip)]
    pos:       ChunkPos,
  },
}
/// A request from the server to the plugin. The server should expect a reply
/// within a certain timeout from the plugin. See also [PluginReply].
#[non_exhaustive]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ServerRequest {
  BlockPlace {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::TypeStore,
  },
  BlockBreak {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::TypeStore,
  },
  ClickWindow {
    slot: i32,
    #[serde(skip)]
    mode: ClickWindow,
  },
  PlayerDamage {
    amount:    f32,
    blockable: bool,
    #[serde(skip)]
    knockback: Vec3,
  },
}

/// A reply from the server to the plugin. This is a response to a
/// [PluginRequest].
#[non_exhaustive]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ServerReply {
  Block {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::TypeStore,
  },
}

fn to_json_ty<T: Clone + Into<U>, U: serde::Serialize, S: serde::Serializer>(
  v: &T,
  ser: S,
) -> Result<S::Ok, S::Error> {
  Into::<U>::into(v.clone()).serialize(ser)
}
