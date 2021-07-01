use crate::player::Player;
use common::{math::ChunkPos, util::UUID};
use std::{
  collections::{hash_map::Values, HashMap},
  iter::Iterator,
  ops::{Deref, DerefMut},
  sync::Arc,
};

pub struct PlayersMap {
  inner: HashMap<UUID, Arc<Player>>,
}

impl PlayersMap {
  pub fn new() -> Self {
    PlayersMap { inner: HashMap::new() }
  }
  pub fn in_range(&self, pos: ChunkPos) -> PlayersIter<'_> {
    PlayersIter { values: self.inner.values(), pos: Some(pos), uuid: None }
  }
}

impl Deref for PlayersMap {
  type Target = HashMap<UUID, Arc<Player>>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl DerefMut for PlayersMap {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

pub struct PlayersIter<'a> {
  values: Values<'a, UUID, Arc<Player>>,
  // The chunk that must be in view
  pos:    Option<ChunkPos>,
  // The uuid that must be skipped
  uuid:   Option<UUID>,
}

impl<'a> Iterator for PlayersIter<'a> {
  type Item = &'a Arc<Player>;

  fn next(&mut self) -> Option<Self::Item> {
    for p in &mut self.values {
      if let Some(uuid) = self.uuid {
        if p.id() == uuid {
          continue;
        }
      }
      if let Some(pos) = self.pos {
        if !p.in_view(pos) {
          continue;
        }
      }
      return Some(p);
    }
    None
  }
}
