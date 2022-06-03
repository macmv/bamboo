use super::WorldManager;
use crate::{item::Stack, player::Click};

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
  pub fn interact(&self, stack: Stack, click: Click) -> EventFlow {
    try_event!(self
      .wm
      .item_behaviors()
      .call(stack.item(), |b| b.interact(click))
      .unwrap_or(Continue));
    Continue
  }
}

impl EventFlow {
  pub fn is_handled(&self) -> bool { matches!(self, Handled) }
}
