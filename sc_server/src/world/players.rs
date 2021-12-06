use crate::player::Player;
use sc_common::{math::ChunkPos, util::UUID};
use std::{
  collections::{
    hash_map::{Keys, Values},
    HashMap,
  },
  ops::{Deref, DerefMut},
  sync::Arc,
};

pub struct PlayersMap {
  inner: HashMap<UUID, Arc<Player>>,
}

pub struct PlayersIter<'a> {
  values: Values<'a, UUID, Arc<Player>>,
  // The chunk that must be in view
  pos:    Option<ChunkPos>,
  // The uuid that must be skipped
  uuid:   Option<UUID>,
}

pub struct KeysIter<'a> {
  keys: Keys<'a, UUID, Arc<Player>>,
}

impl PlayersMap {
  pub fn new() -> Self { PlayersMap { inner: HashMap::new() } }
  pub fn iter(&self) -> PlayersIter<'_> {
    PlayersIter { values: self.inner.values(), pos: None, uuid: None }
  }

  pub fn keys(&self) -> KeysIter<'_> { KeysIter { keys: self.inner.keys() } }

  pub fn get(&self, id: UUID) -> Option<&Arc<Player>> { self.inner.get(&id) }
}

impl Deref for PlayersMap {
  type Target = HashMap<UUID, Arc<Player>>;

  fn deref(&self) -> &Self::Target { &self.inner }
}

impl DerefMut for PlayersMap {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl PlayersIter<'_> {
  pub fn in_view(mut self, pos: ChunkPos) -> Self {
    self.pos = Some(pos);
    self
  }
  pub fn not(mut self, uuid: UUID) -> Self {
    self.uuid = Some(uuid);
    self
  }
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

impl<'a> Iterator for KeysIter<'a> {
  type Item = UUID;

  fn next(&mut self) -> Option<Self::Item> { self.keys.next().map(|v| *v) }
}
