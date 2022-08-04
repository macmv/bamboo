use super::Type;
use crate::{
  block,
  event::EventFlow,
  player::{BlockClick, Click},
};

mod impls;

pub trait Behavior: Send + Sync {
  /// Called when a player right clicks with this item. If `block` is Some, then
  /// the player clicked on a block. Otherwise, they clicked on air.
  ///
  /// If this returns `true`, then the interaction will be cancelled.
  fn interact(&self, click: Click) -> EventFlow {
    let _ = click;
    EventFlow::Continue
  }

  /// Called when the player is about to break a block.
  ///
  /// If this returns `true`, the block will not be broken.
  fn break_block(&self, click: BlockClick) -> EventFlow {
    let _ = click;
    EventFlow::Continue
  }
}

struct DefaultBehavior;
impl Behavior for DefaultBehavior {}

#[derive(Default)]
pub struct BehaviorList {
  behaviors: Vec<Option<Box<dyn Behavior>>>,
}

impl BehaviorList {
  pub fn new() -> Self { BehaviorList::default() }
  // TODO: Use this in plugins with custom blocks
  #[allow(unused)]
  pub fn set(&mut self, ty: Type, imp: Box<dyn Behavior>) {
    while ty.id() as usize >= self.behaviors.len() {
      self.behaviors.push(None);
    }
    self.behaviors[ty.id() as usize] = Some(imp);
  }
  pub fn call<R>(&self, ty: Type, f: impl FnOnce(&dyn Behavior) -> R) -> R {
    bb_server_macros::behavior! {

      ty, f -> :Type:
      DebugStick => impls::DebugStick;
      WaterBucket => impls::Bucket(Some(block::Kind::Water));
      LavaBucket => impls::Bucket(Some(block::Kind::Lava));
      Bucket => impls::Bucket(None);
      Snowball => impls::Snowball;
      Torch => impls::Torch { normal: block::Kind::Torch, wall: block::Kind::WallTorch };
      SoulTorch => impls::Torch { normal: block::Kind::SoulTorch, wall: block::Kind::SoulWallTorch };

      _ => DefaultBehavior;
    }
  }
}
