use super::World;
use crate::player::Player;
use bb_common::{
  math::{ChunkPos, FPos},
  util::{ThreadPool, UUID},
};
use std::{
  collections::{btree_map, BTreeMap, HashMap},
  sync::{Arc, Weak},
};

// A list of chunks that need to be generated. The bool in each entry will be
// set to `true` if a thread in the chunk generator pool is generating this
// chunk. The entry will be removed once the chunk has been loaded. If a chunk
// is being generated, the list can be appended to to have the chunk be sent to
// that client once the chunk is done.
//
// The key is a priority. It is calculated by the distance the player is from a
// chunk. When a player is added to the queue, the
pub struct ChunksToLoad {
  chunks:     BTreeMap<u32, Vec<ChunkToLoad>>,
  priorities: HashMap<ChunkPos, u32>,
}

struct ChunkToLoad {
  pos:        ChunkPos,
  generating: bool,
  players:    HashMap<UUID, PlayerPriority>,
}

struct PlayerPriority {
  player:   Weak<Player>,
  priority: u32,
}

impl ChunksToLoad {
  pub fn new() -> Self { ChunksToLoad { chunks: BTreeMap::new(), priorities: HashMap::new() } }
  fn add_player(&mut self, priority: u32, pos: ChunkPos, player: &Arc<Player>) {
    if let Some(old_priority) = self.priorities.get(&pos) {
      let chunks = self.chunks.get_mut(old_priority).unwrap();
      let mut found = false;
      for chunk in chunks.iter_mut() {
        if chunk.pos == pos {
          found = true;
          chunk
            .players
            .insert(player.id(), PlayerPriority { priority, player: Arc::downgrade(player) });
          break;
        }
      }
      if !found {
        let mut players = HashMap::new();
        players.insert(player.id(), PlayerPriority { priority, player: Arc::downgrade(player) });
        chunks.push(ChunkToLoad { pos, generating: false, players });
      }
      // Update the priority in the `chunks` btree
      let new_priority = old_priority + priority;
      let chunks = self.chunks.remove(old_priority).unwrap();
      match self.chunks.entry(new_priority) {
        btree_map::Entry::Vacant(e) => {
          e.insert(chunks);
        }
        btree_map::Entry::Occupied(mut e) => {
          e.get_mut().extend(chunks);
        }
      }
      // Update the priority in the `priorities` map (replaces the value in `pos`)
      self.priorities.insert(pos, new_priority);
    }
  }
  // If the item has already been removed from the queue, or if there aren't any
  // players waiting on this chunk, we skip it. Returns `true` if this chunk still
  // needs to be generated.
  fn needs_pos(&mut self, pos: ChunkPos) -> bool {
    let priority = match self.priorities.get(&pos) {
      Some(p) => p,
      None => return false,
    };
    let chunks = self.chunks.get_mut(&priority).unwrap();
    let mut needs_pos = true;
    chunks.retain_mut(|chunk| {
      if chunk.pos == pos && chunk.players.is_empty() {
        needs_pos = false
      }
      if chunk.players.is_empty() {
        // Make sure the priorities map doesn't contain invalid references
        self.priorities.remove(&chunk.pos);
        false
      } else {
        true
      }
    });
    needs_pos
  }
  fn remove(&mut self, pos: ChunkPos) -> Option<ChunkToLoad> {
    if let Some(&priority) = self.priorities.get(&pos) {
      let chunks = self.chunks.get_mut(&priority).unwrap();
      for i in 0..chunks.len() {
        if chunks[i].pos == pos {
          let chunk = chunks.remove(i as usize);
          self.priorities.remove(&pos);
          return Some(chunk);
        }
      }
    }
    None
  }
}

impl World {
  pub fn unqueue_chunk(&self, pos: ChunkPos, player: &Player) {
    let mut queue_lock = self.chunks_to_load.lock();
    for chunks in queue_lock.chunks.values_mut() {
      for chunk in chunks {
        if chunk.pos == pos && chunk.players.remove(&player.id()).is_some() {
          break;
        }
      }
    }
  }
  pub fn queue_chunk(&self, pos: ChunkPos, player: &Arc<Player>) {
    {
      let rlock = self.chunks.read();
      if rlock.contains_key(&pos) {
        drop(rlock);
        player.send_chunk(pos, || self.serialize_chunk(pos));
        return;
      }
      // drop rlock
    }
    let mut queue_lock = self.chunks_to_load.lock();
    let priority = player.pos().with_y(0.0).dist(FPos::new(
      pos.block_x() as f64 + 8.0,
      0.0,
      pos.block_z() as f64 + 8.0,
    )) as u32;
    queue_lock.add_player(priority, pos, player);
  }

  pub(super) fn check_chunks_queue(&self, pool: &ThreadPool<super::State>) {
    let mut queue_lock = self.chunks_to_load.lock();
    for (_, chunks) in queue_lock.chunks.iter_mut().rev() {
      for chunk in chunks {
        let pos = chunk.pos;
        if !chunk.generating {};
        // `queue_lock`, so this would deadlock.
        let res = pool.try_execute(move |s| {
          if !s.world.chunks_to_load.lock().needs_pos(pos) {
            return;
          }
          let chunk = s.world.pre_generate_chunk(pos);
          s.world.store_chunks_no_overwrite(vec![(pos, chunk)]);
          let mut queue_lock = s.world.chunks_to_load.lock();
          if let Some(chunk) = queue_lock.remove(pos) {
            if !chunk.players.is_empty() {
              let out = s.world.serialize_chunk(pos);
              for player in chunk.players.values() {
                if let Some(p) = player.player.upgrade() {
                  p.send_chunk(pos, || out.clone());
                }
              }
            }
          }
        });
        match res {
          Ok(()) => chunk.generating = true,
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
