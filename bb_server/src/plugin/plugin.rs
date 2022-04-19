use super::{JsonBlock, JsonPlayer, JsonPos};
use crate::{block, player::Player};
use bb_common::{config::Config, math::Pos, net::sb::ClickWindow};
use std::{error::Error, fmt, sync::Arc};

#[cfg(feature = "panda_plugins")]
use super::panda::PandaPlugin;

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
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PluginEvent {
  Register { ty: String },
  Ready,

  SendChat { text: String },
}
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PluginRequest {
  GetBlock { pos: JsonPos },
}

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
  Reply {
    reply_id: u32,
    #[serde(flatten)]
    reply:    ServerReply,
  },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
  BlockPlace {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::Type,
  },
  BlockBreak {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::Type,
  },
  Chat {
    text: String,
  },
  ClickWindow {
    slot: i32,
    #[serde(skip)]
    mode: ClickWindow,
  },
  PlayerJoin,
  PlayerLeave,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum GlobalServerEvent {
  Tick,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ServerReply {
  Block {
    #[serde(serialize_with = "to_json_ty::<_, JsonPos, _>")]
    pos:   Pos,
    #[serde(serialize_with = "to_json_ty::<_, JsonBlock, _>")]
    block: block::Type,
  },
}

pub trait PluginImpl: std::any::Any {
  /// If this returns an error, the plugin will be removed, and this function
  /// will not be called again.
  ///
  /// If this returns `false`, the event will be cancelled.
  fn call(&self, event: ServerMessage) -> Result<bool, CallError>;
  #[cfg(feature = "panda_plugins")]
  fn panda(&mut self) -> Option<&mut PandaPlugin> { None }
}

pub struct Plugin {
  #[allow(unused)]
  config: Config,
  imp:    Box<dyn PluginImpl + Send + Sync>,
}

#[derive(Debug)]
pub struct CallError {
  pub keep:  bool,
  pub inner: Box<dyn Error>,
}

impl CallError {
  pub fn no_keep(inner: impl Error + 'static) -> Self {
    CallError { keep: false, inner: Box::new(inner) }
  }
  pub fn keep(inner: impl Error + 'static) -> Self {
    CallError { keep: true, inner: Box::new(inner) }
  }
}

impl fmt::Display for CallError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.inner)?;
    if self.keep {
      write!(f, " (plugin still valid)")
    } else {
      write!(f, " (plugin no longer valid)")
    }
  }
}

impl Error for CallError {}

impl Plugin {
  pub fn new(config: Config, imp: impl PluginImpl + Send + Sync + 'static) -> Self {
    Plugin { config, imp: Box::new(imp) }
  }
  pub fn call(&self, ev: ServerMessage) -> Result<bool, CallError> { self.imp.call(ev) }
  #[cfg(feature = "panda_plugins")]
  pub fn unwrap_panda(&mut self) -> &mut PandaPlugin { self.imp.panda().unwrap() }
}

fn to_json_ty<T: Clone + Into<U>, U: serde::Serialize, S: serde::Serializer>(
  v: &T,
  ser: S,
) -> Result<S::Ok, S::Error> {
  Into::<U>::into(v.clone()).serialize(ser)
}
