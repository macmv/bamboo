mod attack;
mod behavior;
mod dig;
mod inventory;
mod shared;
mod stack;
mod ty;
mod ui;
mod version;

pub use behavior::Behavior;
pub use inventory::{Inventory, SingleInventory, WrappedInventory};
pub use shared::SharedInventory;
pub use stack::Stack;
pub use ty::{Data, Type};
pub use ui::{UIError, UI};
pub use version::TypeConverter;

use behavior::BehaviorList;

pub struct BehaviorStore {
  behaviors: BehaviorList,
}

impl BehaviorStore {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { BehaviorStore { behaviors: behavior::make_behaviors() } }
  pub fn call<R>(&self, ty: Type, f: impl FnOnce(&dyn Behavior) -> R) -> Option<R> {
    self.behaviors.get(ty).map(f)
  }
}
