use crate::sync::{LazyGuard, LazyLock};
use bb_common::util::UUID;
use std::{any::Any, collections::HashMap};

pub trait PlayerStore: Any + Send {
  fn as_any(&mut self) -> &mut dyn Any;
  fn new() -> Self
  where
    Self: Sized;
}

pub struct PluginStore {
  players: HashMap<UUID, Box<dyn PlayerStore>>,
}

impl PluginStore {
  fn new() -> PluginStore { PluginStore { players: HashMap::new() } }
  pub fn player<T: PlayerStore>(&mut self, id: UUID) -> &mut T {
    let b = self.players.entry(id).or_insert_with(|| Box::new(T::new()));
    b.as_any().downcast_mut().expect("wrong type given for player store")
  }
}

static STORE: LazyLock<PluginStore> = LazyLock::new(|| PluginStore::new());
pub fn store<'a>() -> LazyGuard<'a, PluginStore> { STORE.lock() }
