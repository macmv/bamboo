//! There are six kinds of messages:
//!
//! - [PluginMessage], which stores all the below types:
//!   - [PluginEvent], for a message that doesn't need a reply.
//!   - [PluginRequest], for a message that needs the server to reply.
//!   - [PluginReply], for a reply to a message from the server.
//! - [ServerMessage], which stores all the below types:
//!   - [ServerEvent], for a message that doesn't need a reply.
//!   - [ServerRequest], for a message that needs the plugin to reply.
//!   - [ServerReply], for a reply to a message from the plugin.
//!
//! # Examples
//!
//! ```text
//!     Server <-- PluginRequest::GetBlock Plugin
//!     Server   ServerReply::Block -->    Plugin
//! ```
//! ```text
//!     Server ServerRequest::PlaceBlock --> Plugin
//!     Server   <-- PluginReply::Cancel     Plugin
//! ```
//! ```text
//!     Server <-- ServerRequest::Chat Plugin
//! ```

mod json;
mod types;
mod world;

use crate::{
  block,
  math::Vec3,
  player::{Click, Player},
  world::{MultiChunk, WorldManager},
};
use bb_common::{
  math::{ChunkPos, Pos},
  net::sb::ClickWindow,
  util::{Chat, Hand},
};
use parking_lot::Mutex;
use std::sync::Arc;

pub use types::*;

pub struct Events<'a> {
  wm: &'a WorldManager,
}
pub enum EventFlow {
  Handled,
  Continue,
}

macro_rules! try_event {
  ( $expr:expr ) => {
    match $expr {
      EventFlow::Continue => EventFlow::Continue,
      EventFlow::Handled => return EventFlow::Handled,
    }
  };
}

use EventFlow::*;
impl Events<'_> {
  pub fn interact(&self, player: &Arc<Player>, _hand: Hand, click: Click) -> EventFlow {
    // self.req(player, ServerRequest::Interact { hand, click });
    let stack = player.lock_inventory().main_hand().clone();
    try_event!(self.player_request(player.clone(), Interact { slot: 36 }));
    try_event!(self.wm.item_behaviors().call(stack.item(), |i| i.interact(click)));
    if let Click::Block(click) = click {
      try_event!(self
        .wm
        .block_behaviors()
        .call(click.block.kind(), |b| b.interact(click.block, player)));
    }
    Continue
  }

  /// Send a [`GlobalEvent`]. All plugins will receive this event, and will not
  /// be able to cancel it.
  pub fn global_event(&self, ev: impl Into<GlobalEvent>) {
    self.wm.plugins().global_event(ev.into());
  }
  /// Send an [`Event`]. All plugins will receive this event, and cannot cancel
  /// it.
  pub fn player_event(&self, player: Arc<Player>, ev: impl Into<PlayerEvent>) {
    self.wm.plugins().event(player, ev.into());
  }
  /// Send a [`Request`]. All plugins will receive this event, and can cancel
  /// it.
  pub fn player_request(&self, player: Arc<Player>, req: impl Into<PlayerRequest>) -> EventFlow {
    self.wm.plugins().req(player, req.into())
  }
}

impl EventFlow {
  pub fn is_handled(&self) -> bool { matches!(self, Handled) }
  pub fn is_continue(&self) -> bool { matches!(self, Continue) }
}
