use super::json::*;
use crate::{block, math::Vec3, player::Player, plugin::IntoPanda};
use bb_common::{
  math::{ChunkPos, Pos},
  net::sb::ClickWindow,
};
use panda::{define_ty, docs::markdown, runtime::Var, Panda};

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
    #[serde(flatten)]
    event: PlayerEvent,
  },
  PlayerRequest {
    reply_id: u32,
    #[serde(flatten)]
    request:  PlayerRequest,
  },
  Reply {
    reply_id: u32,
    #[serde(flatten)]
    reply:    ServerReply,
  },
}

macro_rules! define_event {
  (
    $name:ident,
    $str_name:literal,
    $event_name:ident,
    $(
      $( #[$attr:meta] )*
      $field:ident: $ty:ty,
    )*
  ) => {
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct $name {
      $(
        $( #[$attr] )*
        pub $field: $ty,
      )*
    }

    #[define_ty(prefix = "bamboo::event")]
    impl $name {
      $(
        #[field]
        fn $field(&self) -> <$ty as IntoPanda>::Panda {
          self.$field.clone().into_panda()
        }
      )*
    }

    impl From<$name> for $event_name {
      fn from(v: $name) -> Self {
        $event_name::$name(v)
      }
    }

    impl IntoPanda for $name {
      type Panda = Self;
      fn into_panda(self) -> Self {
        self
      }
    }
  }
}

macro_rules! define_events {
  (
    $event_name:ident,
    { $(
      $( #[$extra_attr:meta] )*
      $extra:ident: $extra_ty:ty,
    )* },
    $name:ident: $str_name:literal {
      $(
        $( #[$attr:meta] )*
        $field:ident: $ty:ty,
      )*
    },
    $( $args:tt )*
  ) => {
    define_event!($name, $str_name, $event_name, $( $( #[$extra_attr] )* $extra: $extra_ty, )* $( $( #[$attr] )* $field: $ty, )*);
    define_events!(
      $event_name,
      { $( $( #[$extra_attr] )* $extra: $extra_ty, )* },
      $( $args )*
    );
  };
  (
    $event_name:ident,
    { $(
      $( #[$extra_attr:meta] )*
      $extra:ident: $extra_ty:ty,
    )* },
  ) => {}
}

macro_rules! event {
  (
    $( #[$event_attr:meta] )*
    $event_name:ident: { $(
      $( #[$extra_attr:meta] )*
      $extra:ident: $extra_ty:ty
    )* }
    $(
      $( #[$specific_event_attr:meta] )*
      $name:ident: $str_name:literal {
        $( $args:tt )*
      },
    )*
  ) => {
    define_events!($event_name, { $( $( #[$extra_attr] )* $extra: $extra_ty,)* }, $( $name: $str_name { $( $args )* }, )*);

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
      pub fn add_builtins(pd: &mut Panda) {
        $(
          pd.def_callback($str_name, markdown!($( #[$specific_event_attr] )*));
          pd.add_builtin_ty::<$name>();
        )*
      }
    }
    impl IntoPanda for $event_name {
      type Panda = Var;
      fn into_panda(self) -> Var {
        match self {
          $(
            Self::$name(v) => v.into_panda().into(),
          )*
        }
      }
    }
  }
}

event! {
  /// An event from the server to the plugin. There is also a player listed with
  /// this event.
  PlayerEvent: {
    #[serde(serialize_with = "to_json_ty::<_, JsonPlayer, _>")]
    player: Arc<Player>
  }

  /// Called when a chat message is sent by a player.
  Chat: "chat" { text: String, },
  PlayerJoin: "player_join" {},
  PlayerLeave: "player_leave" {},
}
event! {
  /// An event from the server to the plugin. This is very similar to
  /// [ServerEvent], but there is no player specified with this event.
  GlobalEvent: {}

  /// Called every server tick.
  Tick: "tick" {},
  GenerateChunk: "generate_chunk" {
    generator: String,
    // #[serde(skip)]
    // chunk:     Arc<Mutex<MultiChunk>>,
    #[serde(skip)]
    pos:       ChunkPos,
  },
}

event! {
  /// A request from the server to the plugin. The server should expect a reply
  /// within a certain timeout from the plugin. See also [PluginReply].
  PlayerRequest: {
    #[serde(skip)]
    player: Arc<Player>
  }

  /// Called every time a client places a block.
  BlockPlace: "block_place" {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::TypeStore,
  },
  /// Called every time a client breaks a block.
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
