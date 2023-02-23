use super::json::*;
use crate::{block, item::Stack, math::Vec3, player::Player, plugin::IntoPanda};
use bb_common::{
  math::{ChunkPos, FPos, Pos},
  net::sb::ClickWindow,
  util::GameMode,
};
use panda::{
  define_ty,
  docs::markdown,
  runtime::{PandaType, Var, VarSend},
  Panda,
};

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
/// A response to a request from the server. See also [`PlayerRequest`].
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
    #[cfg_attr(feature = "python_plugins", ::pyo3::pyclass)]
    pub struct $name {
      $(
        $( #[$attr] )*
        #[serde(skip)]
        pub $field: $ty,
      )*
    }

    // TODO: Need to swith to proc macro to get the functionality we need with #[getter] :(
    #[define_ty(prefix = "bamboo::event")]
    // #[cfg_attr(feature = "python_plugins", ::pyo3::pymethods)]
    impl $name {
      $(
        $( #[$attr] )*
        // #[getter]
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

macro_rules! add_builtins {
  (
    $pd:ident;
    $( $extra_arg:ty ),*;
    [ $str_name:expr, $name:ident, $( #[$specific_event_attr:meta] )* ]
    $( $extra:tt )*
  ) => {
    $pd.def_callback($str_name, vec![$name::var_type() $(, <$extra_arg>::var_type() )*], markdown!($( #[$specific_event_attr] )*));
    $pd.add_builtin_ty::<$name>();
    add_builtins!($pd; $( $extra_arg ),*; $( $extra )*);
  };
  (
    $pd:ident;
    $( $extra_arg:ty ),*;
  ) => {};
}

macro_rules! event {
  (
    $( #[$event_attr:meta] )*
    $event_name:ident: { $(
      $( #[$extra_attr:meta] )*
      $extra:ident: $extra_ty:ty
    )* } -> ( $( $extra_arg:ty ),* )
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
        add_builtins!(pd; $( $extra_arg ),*; $( [ $str_name, $name, $( #[$specific_event_attr] )* ] )*);
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
    #[cfg(feature = "python_plugins")]
    impl $event_name {
      pub fn with_python<R>(self, py: pyo3::Python, f: impl FnOnce((pyo3::PyObject,)) -> R) -> R {
        use pyo3::IntoPy;
        match self {
          $(
            Self::$name(v) => f((v.into_py(py),)),
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
    // #[serde(serialize_with = "to_json_ty::<_, JsonPlayer, _>")]
    player: Arc<Player>
  } -> ()

  /// Called when a chat message is sent by a player.
  PlayerJoin: "player_join" {},
  PlayerLeave: "player_leave" {},
  PlayerMove: "player_move" {
    old_pos: FPos,
    new_pos: FPos,
  },
}
event! {
  /// An event from the server to the plugin. This is very similar to
  /// [`PlayerEvent`], but there is no player specified with this event.
  GlobalEvent: {} -> ()

  /// Called every server tick.
  Tick: "tick" {},
  GenerateChunk: "generate_chunk" {
    generator: String,
    // #[serde(skip)]
    // chunk:     Arc<Mutex<MultiChunk>>,
    pos:       ChunkPos,
  },
}

event! {
  /// A request from the server to the plugin. The server should expect a reply
  /// within a certain timeout from the plugin.
  PlayerRequest: {
    /// The player for this event.
    player: Arc<Player>
  } -> (crate::plugin::types::event::PEventFlow)

  /// Called every time a client breaks a block.
  BlockBreak: "block_break" {
    /// The position of the block being broken.
    pos:   Pos,
    /// The type of the block being broken.
    block: block::TypeStore,
  },
  /// Called every time a client places a block.
  BlockPlace: "block_place" {
    /// The position of the block that was clicked on.
    clicked_pos: Pos,
    /// The position of the block being placed. This will be one block away from
    /// the clicked block, or the same as the clicked block for blocks that are
    /// replacable (such as tall grass).
    placed_pos:   Pos,
    /// The type of the block being placed.
    block: block::TypeStore,
  },
  /// Called when a client clicks on an item in an inventory.
  ClickWindowEvent: "click_window" {
    /// The inventory slot of the click.
    slot: i32,
    /// The type of window click.
    mode: ClickWindow,
  },
  /// Called when a player switches game mode.
  ///
  /// This can be from commands or another plugin switching the player's
  /// game mode. Cancelling this will undo the game mode switch.
  ChangeGameMode: "change_game_mode" {
    /// The player's game mode before the switch.
    old_mode: GameMode,
    /// The player's game mode after the switch.
    new_mode: GameMode,
  },
  /// Called when a client sends a chat message.
  ///
  /// Commands are parsed seperately, and will not trigger this event.
  Chat: "chat" {
    /// The entire contents of the chat message, as entered on the client.
    text: String,
  },
  /// Called when a client sends a command.
  ///
  /// Generally, commands should be handled by the parser function given
  /// when creating the command. However, if you need to cancel another
  /// plugin's command, or override some functionality, this will work.
  CommandSent: "command" {
    /// The arguments to the command. This includes a literal at the start
    /// which is the command name.
    args: Vec<VarSend>,
  },
  /// Called when a player interacts with something.
  ///
  /// This could be a player right clicking or left clicking on a block
  /// or an entity.
  Interact: "interact" {
    /// The slot the player interacted with.
    ///
    /// TODO: Fix.
    slot: i32,
  },
  /// Called when a player drops an item.
  ///
  /// Cancelling this event will keep the item in their inventory.
  ItemDrop: "item_drop" {
    /// The stack that the player is dropping.
    stack: Stack,
    /// This will be `true` if the player dropped the entire stack.
    /// If there is only one item in the stack, then this can either
    /// be `true` or `false`.
    full_stack: bool,
  },
  /// Called when a player is damaged.
  ///
  /// Cancelling this will cause the player to not be damaged at all.
  PlayerDamage: "player_damage" {
    /// The amount of damage the player is being hit by. This is not
    /// the amount of health they will lose, as armor and other effects
    /// will decrease this amount.
    amount:    f32,
    /// If true, then armor and other effects will decrease the amount
    /// of damage. If false, the `amount` is the direct amount delt to
    /// the player's health.
    ///
    /// This is mostly used for splash potitions of instant damage.
    blockable: bool,
    /// The knockback vector. This will be added to the player's velocity
    /// after damage is applied.
    knockback: Vec3,
  },
  /// Called when the server receives a packet from a client.
  ///
  /// Cancelling this packet will make it appear as if the packet were
  /// never sent.
  ReceivePacket: "packet" {
    data: String,
  },
}

/// A reply from the server to the plugin.
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
