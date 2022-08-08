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
    try_event!(self.req(player.clone(), ServerRequest::Interact { slot: 36 }));
    try_event!(self.wm.item_behaviors().call(stack.item(), |i| i.interact(click)));
    if let Click::Block(click) = click {
      try_event!(self
        .wm
        .block_behaviors()
        .call(click.block.kind(), |b| b.interact(click.block, player)));
    }
    Continue
  }

  fn global_event(&self, ev: GlobalServerEvent) { self.wm.plugins().global_event(ev); }
  fn req(&self, player: Arc<Player>, req: ServerRequest) -> EventFlow {
    self.wm.plugins().req(player, req)
  }
  fn event(&self, player: Arc<Player>, ev: ServerEvent) { self.wm.plugins().event(player, ev); }

  pub fn tick(&self) { self.global_event(GlobalServerEvent::Tick); }
  pub fn generate_chunk(&self, generator: &str, chunk: Arc<Mutex<MultiChunk>>, pos: ChunkPos) {
    self.global_event(GlobalServerEvent::GenerateChunk { generator: generator.into(), chunk, pos });
  }
  pub fn block_place(&self, player: Arc<Player>, pos: Pos, block: block::Type) -> bool {
    self.req(player, ServerRequest::BlockPlace { pos, block: block.to_store() });
    true
  }
  pub fn block_break(&self, player: Arc<Player>, pos: Pos, block: block::Type) -> bool {
    self.req(player, ServerRequest::BlockBreak { pos, block: block.to_store() });
    true
  }
  pub fn click_window(&self, player: Arc<Player>, slot: i32, mode: ClickWindow) -> bool {
    self.req(player, ServerRequest::ClickWindow { slot, mode });
    true
  }
  pub fn chat_message(&self, player: Arc<Player>, message: Chat) {
    self.event(player, ServerEvent::Chat { text: message.to_plain() });
  }
  pub fn player_join(&self, player: Arc<Player>) { self.event(player, ServerEvent::PlayerJoin {}); }
  pub fn player_damage(
    &self,
    player: Arc<Player>,
    amount: f32,
    blockable: bool,
    knockback: Vec3,
  ) -> bool {
    self.req(player, ServerRequest::PlayerDamage { amount, blockable, knockback });
    true
  }
  pub fn use_item(&self, player: Arc<Player>, slot: i32) -> EventFlow {
    self.req(player, ServerRequest::Interact { slot })
  }
  pub fn player_leave(&self, player: Arc<Player>) {
    self.event(player, ServerEvent::PlayerLeave {});
  }
}

impl EventFlow {
  pub fn is_handled(&self) -> bool { matches!(self, Handled) }
}
