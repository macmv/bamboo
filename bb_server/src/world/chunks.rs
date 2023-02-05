use super::World;
use crate::player::Player;
use bb_common::{
  math::{ChunkPos, FPos},
  util::{ThreadPool, UUID},
};
use std::{
  collections::HashMap,
  sync::{Arc, Weak},
};

// A list of chunks that need to be generated. The bool in each entry will be
// set to `true` if a thread in the chunk generator pool is generating this
// chunk. The entry will be removed once the chunk has been loaded. If a chunk
// is being generated, the list can be appended to to have the chunk be sent to
// that client once the chunk is done.
pub struct ChunksToLoad {
  chunks:       HashMap<ChunkPos, ChunkToLoad>,
  // A sorted list of chunks to load. This is cached, and only updated once per tick.
  sorted:       Vec<ChunkToLoad>,
  needs_update: bool,
}

#[derive(Clone)]
struct ChunkToLoad {
  pos:        ChunkPos,
  generating: bool,
  players:    HashMap<UUID, Weak<Player>>,
}

impl ChunksToLoad {
  pub fn new() -> Self {
    ChunksToLoad { chunks: HashMap::new(), sorted: vec![], needs_update: false }
  }
  // If the item has already been removed from the queue, or if there aren't any
  // players waiting on this chunk, we skip it. Returns `true` if this chunk still
  // needs to be generated.
  fn needs_pos(&mut self, pos: ChunkPos) -> bool {
    match self.chunks.get(&pos) {
      Some(c) => !c.players.is_empty(),
      None => false,
    }
  }
  fn add(&mut self, pos: ChunkPos, player: &Arc<Player>) {
    if let Some(c) = self.chunks.get_mut(&pos) {
      c.players.insert(player.id(), Arc::downgrade(player));
      return;
    }
    let mut players = HashMap::new();
    players.insert(player.id(), Arc::downgrade(player));
    self.chunks.insert(pos, ChunkToLoad { pos, generating: false, players });
    // Cache should always get updated here, as the position of `player` has
    // changed, so all the priorities need to be recalculated.
    self.needs_update = true;
  }
  fn remove_pos(&mut self, pos: ChunkPos) -> Option<ChunkToLoad> {
    self.chunks.remove(&pos).map(|c| {
      // Don't resort here, just remove the one element
      if let Some(i) = self.sorted.iter().position(|inner| inner.pos == c.pos) {
        self.sorted.remove(i);
      } else {
        self.needs_update = true;
      }
      c
    })
  }
  fn remove_player(&mut self, pos: ChunkPos, player: &Player) {
    if let Some(chunk) = self.chunks.get_mut(&pos) {
      chunk.players.remove(&player.id());
      if chunk.players.is_empty() {
        self.remove_pos(pos);
      } else {
        // Don't resort here, just update the one element
        if let Some(i) = self.sorted.iter().position(|inner| inner.pos == pos) {
          self.sorted[i].players.remove(&player.id());
        } else {
          self.needs_update = true;
        }
      }
    }
  }
  fn update_cache(&mut self) {
    if self.needs_update {
      self.needs_update = false;
      // Don't reallocate `sorted`
      self.sorted.clear();
      self.sorted.extend(self.chunks.values().cloned());
      self.sorted.sort_unstable_by_key(|chunk| {
        let mut priority = 0;
        for weak in chunk.players.values() {
          if let Some(player) = weak.upgrade() {
            priority += player.pos().with_y(0.0).dist(FPos::new(
              chunk.pos.block_x() as f64 + 8.0,
              0.0,
              chunk.pos.block_z() as f64 + 8.0,
            )) as u32;
          }
        }
        priority
      });
    }
  }
}

impl World {
  pub fn unqueue_chunk(&self, pos: ChunkPos, player: &Player) {
    self.chunks_to_load.lock().remove_player(pos, player);
  }
  pub fn queue_chunk(&self, pos: ChunkPos, player: &Arc<Player>) {
    if self.regions.region(pos, || self.new_chunk(), |region| region.has_chunk(pos)) {
      player.send_chunk(pos, || self.serialize_chunk(pos).into());
      return;
    }
    self.chunks_to_load.lock().add(pos, player);
  }

  pub(super) fn check_chunks_queue(&self, pool: &ThreadPool<super::State>) {
    let mut queue_lock = self.chunks_to_load.lock();
    queue_lock.update_cache();
    for i in 0..queue_lock.sorted.len() {
      let chunk = &mut queue_lock.sorted[i];
      if !chunk.generating {
        let pos = chunk.pos;
        // `queue_lock`, so this would deadlock.
        let res = pool.try_execute(move |s| {
          if !s.world.chunks_to_load.lock().needs_pos(pos) {
            return;
          }
          let chunk = s.world.pre_generate_chunk(pos);
          s.world.store_chunks_no_overwrite(vec![(pos, chunk)]);
          let mut queue_lock = s.world.chunks_to_load.lock();
          if let Some(chunk) = queue_lock.remove_pos(pos) {
            if !chunk.players.is_empty() {
              let out = s.world.serialize_chunk(pos);
              for weak in chunk.players.values() {
                if let Some(p) = weak.upgrade() {
                  p.send_chunk(pos, || out.clone().into());
                }
              }
            }
          }
        });
        match res {
          Ok(()) => {
            chunk.generating = true;
            queue_lock.chunks.get_mut(&pos).unwrap().generating = true;
          }
          // If the channel is full, then we have at least 256 chunks queued. In this
          // case, we just wait until some of them are done. The cache will be cleared out
          // when the player leaves or moves to another area, so it shouldn't be a problem in
          // most cases.
          Err(_) => break,
        }
      }
    }
  }
}
