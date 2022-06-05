use bb_common::util::UUID;
use parking_lot::{lock_api::RawMutex, MappedMutexGuard, Mutex, MutexGuard};
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

static STORE: Mutex<Option<PluginStore>> = Mutex::const_new(parking_lot::RawMutex::INIT, None);

pub fn store<'a>() -> MappedMutexGuard<'a, PluginStore> {
  loop {
    if let Some(mut lock) = STORE.try_lock() {
      if lock.is_none() {
        *lock = Some(PluginStore::new());
      }
      break MutexGuard::map(lock, |opt| opt.as_mut().unwrap());
    }
  }
}
