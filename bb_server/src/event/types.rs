use super::json::*;
use crate::{block, math::Vec3, player::Player, plugin::IntoPanda, world::MultiChunk};
use bb_common::{
  math::{ChunkPos, Pos},
  net::sb::ClickWindow,
};
use bb_server_macros::define_ty;
use panda::runtime::Var;
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
  GlobalEvent {
    #[serde(flatten)]
    event: GlobalEvent,
  },
  PlayerEvent {
    #[serde(serialize_with = "to_json_ty::<_, JsonPlayer, _>")]
    player: Arc<Player>,
    #[serde(flatten)]
    event:  PlayerEvent,
  },
  PlayerRequest {
    reply_id: u32,
    #[serde(serialize_with = "to_json_ty::<_, JsonPlayer, _>")]
    player:   Arc<Player>,
    #[serde(flatten)]
    request:  PlayerRequest,
  },
  Reply {
    reply_id: u32,
    #[serde(flatten)]
    reply:    ServerReply,
  },
}

macro_rules! event {
  (
    $( #[$event_attr:meta] )*
    $event_name:ident:
    $(
      $name:ident: $str_name:literal {
        $(
          $( #[$attr:meta] )*
          $field:ident: $ty:ty,
        )*
      },
    )*
  ) => {
    $(
      #[derive(Debug, Clone, serde::Serialize)]
      pub struct $name {
        $(
          $( #[$attr] )*
          $field: $ty,
        )*
      }

      #[define_ty]
      impl $name {
        $(
          #[field]
          fn $field(&self) -> &$ty {
            &self.$field
          }
        )*
      }

      impl From<$name> for $event_name {
        fn from(v: $name) -> Self {
          $event_name::$name(v)
        }
      }

      impl IntoPanda for $name {
        fn into_panda(&self) -> Var {
          self.into()
        }
      }
    )*
    $( #[$event_attr] )*
    #[non_exhaustive]
    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(tag = "type")]
    pub enum $event_name {
      $(
        $name($name),
      )*
    }
    impl $event_name {
      pub fn name(&self) -> &'static str {
        match self {
          $(
            Self::$name(_) => $str_name,
          )*
        }
      }
      pub fn all_names() -> &'static [&'static str] {
        &[$( $str_name, )*]
      }
    }
    impl IntoPanda for $event_name {
      fn into_panda(&self) -> Var {
        match self {
          $(
            Self::$name(v) => v.into_panda(),
          )*
        }
      }
    }
  }
}

event! {
  /// An event from the server to the plugin. There is also a player listed with
  /// this event.
  PlayerEvent:
  Chat: "chat" { text: String, },
  PlayerJoin: "player_join" {},
  PlayerLeave: "player_leave" {},
}
event! {
  /// An event from the server to the plugin. This is very similar to
  /// [ServerEvent], but there is no player specified with this event.
  GlobalEvent:
  Tick: "tick" {},
  GenerateChunk: "generate_chunk" {
    generator: String,
    #[serde(skip)]
    chunk:     Arc<Mutex<MultiChunk>>,
    #[serde(skip)]
    pos:       ChunkPos,
  },
}

event! {
  /// A request from the server to the plugin. The server should expect a reply
  /// within a certain timeout from the plugin. See also [PluginReply].
  PlayerRequest:
  BlockPlace: "block_place" {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::TypeStore,
  },
  BlockBreak: "block_break" {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::TypeStore,
  },
  ClickWindowEvent: "click_window" {
    slot: i32,
    #[serde(skip)]
    mode: ClickWindow,
  },
  PlayerDamage: "player_damage" {
    amount:    f32,
    blockable: bool,
    #[serde(skip)]
    knockback: Vec3,
  },
  Interact: "interact" {
    slot: i32,
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
