use super::Type;
use crate::{block::Block, player::Player};
use std::sync::Arc;

mod impls;

pub trait Behavior: Send + Sync {
  /// Called when a player right clicks with this item on a block.
  ///
  /// If this returns `true`, then the interaction will be cancelled.
  fn interact_block(&self, block: Block, player: &Arc<Player>) -> bool {
    let _ = (block, player);
    false
  }
}

#[derive(Default)]
pub struct BehaviorList {
  behaviors: Vec<Option<Box<dyn Behavior>>>,
}

impl BehaviorList {
  pub fn new() -> Self { BehaviorList::default() }
  pub fn set(&mut self, ty: Type, imp: Box<dyn Behavior>) {
    while ty.id() as usize >= self.behaviors.len() {
      self.behaviors.push(None);
    }
    self.behaviors[ty.id() as usize] = Some(imp);
  }
  pub fn get(&self, ty: Type) -> Option<&dyn Behavior> {
    match self.behaviors.get(ty.id() as usize) {
      Some(Some(b)) => Some(b.as_ref()),
      _ => None,
    }
  }
}

pub fn make_behaviors() -> BehaviorList {
  let mut out = BehaviorList::new();
  bb_plugin_macros::behavior! {
    :Type:

    DebugStick => impls::DebugStick;
  };
  out
}
