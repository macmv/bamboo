//! There are three kinds of messages:
//!
//! - [GlobalEvent], for a non-cancellable event.
//! - [PlayerEvent], for a non-cancellable event with a player.
//! - [PlayerRequest], for a cancellable event with a player.

mod json;
mod types;
mod world;

use crate::{
  player::{Click, Player},
  world::WorldManager,
};
use bb_common::util::Hand;

use std::sync::Arc;

pub use types::*;

pub struct Events<'a> {
  wm: &'a WorldManager,
}
#[derive(Debug, Clone, Copy)]
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
    try_event!(self.player_request(Interact { player: player.clone(), slot: 36 }));
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
  /// Send an [`PlayerEvent`]. All plugins will receive this event, and cannot
  /// cancel it.
  pub fn player_event(&self, ev: impl Into<PlayerEvent>) {
    self.wm.plugins().player_event(ev.into());
  }
  /// Send a [`PlayerRequest`]. All plugins will receive this event, and can
  /// cancel it.
  pub fn player_request(&self, req: impl Into<PlayerRequest>) -> EventFlow {
    self.wm.plugins().player_request(req.into())
  }
}

impl EventFlow {
  pub fn is_handled(&self) -> bool { matches!(self, Handled) }
  pub fn is_continue(&self) -> bool { matches!(self, Continue) }
}
