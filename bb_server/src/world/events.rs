use super::WorldManager;
use crate::player::{Click, Player};
use bb_common::util::Hand;
use std::sync::Arc;

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

impl WorldManager {
  pub fn events(&self) -> Events { Events { wm: self } }
}

use EventFlow::*;
impl Events<'_> {
  pub fn interact(&self, player: &Arc<Player>, _hand: Hand, click: Click) -> EventFlow {
    let stack = player.lock_inventory().main_hand().clone();
    try_event!(self
      .wm
      .item_behaviors()
      .call(stack.item(), |i| i.interact(click))
      .unwrap_or(Continue));
    if let Click::Block(click) = click {
      try_event!(self
        .wm
        .block_behaviors()
        .call(click.block.kind(), |b| b.interact(click.block, player))
        .unwrap_or(Continue));
    }
    Continue
  }
}

impl EventFlow {
  pub fn is_handled(&self) -> bool { matches!(self, Handled) }
}
