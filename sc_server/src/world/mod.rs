mod blocks;
pub mod chunk;
mod entities;
pub mod gen;
mod init;
mod players;
pub mod schematic;

use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard};
use sc_common::{
  config::Config,
  math::{ChunkPos, FPos, Pos},
  net::cb,
  util::{
    chat::{Chat, Color},
    ThreadPool, UUID,
  },
  version::ProtocolVersion,
};
use std::{
  collections::{HashMap, HashSet},
  convert::TryInto,
  sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
    Arc,
  },
  thread,
  time::{Duration, Instant},
};

use crate::{
  block, command::CommandTree, entity, entity::Entity, item, net::ConnSender, player::Player,
  plugin,
};
use chunk::MultiChunk;
use gen::WorldGen;

pub use players::{PlayersIter, PlayersMap};

// pub struct ChunkRef<'a> {
//   pos:    ChunkPos,
//   // Need to keep this is scope while we mess with the chunk
//   chunks: RwLockReadGuard<'a, HashMap<ChunkPos, Arc<Mutex<MultiChunk>>>>,
// }
//
// impl ChunkRef<'_> {
//   fn lock<'a>(&'a self) -> MutexGuard<'a, MultiChunk> {
//     self.chunks.get(&self.pos).unwrap().lock().unwrap()
//   }
// }

/// A chunk in the world with a number of people viewing it. If the count is at
/// 0, then this chunk is essentially flagged for unloading. Chunks are unloaded
/// lazily, so this chunk will just end up being cleaned up in the future.
pub struct CountedChunk {
  count: AtomicU32,
  chunk: Mutex<MultiChunk>,
}

impl CountedChunk {
  /// Creates a new counted chunk with the counter at 0.
  pub fn new(c: MultiChunk) -> CountedChunk {
    CountedChunk { count: 0.into(), chunk: Mutex::new(c) }
  }
}

pub struct World {
  chunks:            RwLock<HashMap<ChunkPos, CountedChunk>>,
  // Whenever we want to unload chunks, we will clear out this map. So there is no situation where
  // a rwlock is more useful than a normal mutex.
  unloadable_chunks: Mutex<HashSet<ChunkPos>>,
  gen:               WorldGen,
  players:           RwLock<PlayersMap>,
  entities:          RwLock<HashMap<i32, Arc<Entity>>>,
  eid:               AtomicI32,
  block_converter:   Arc<block::TypeConverter>,
  item_converter:    Arc<item::TypeConverter>,
  entity_converter:  Arc<entity::TypeConverter>,
  plugins:           Arc<plugin::PluginManager>,
  commands:          Arc<CommandTree>,
  mspt:              Arc<AtomicU32>,
  wm:                Arc<WorldManager>,
  // If set, then the world cannot be modified.
  locked:            AtomicBool,
}

pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds:           Mutex<Vec<Arc<World>>>,
  // Player id to world index
  players:          Mutex<HashMap<UUID, usize>>,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
  commands:         Arc<CommandTree>,
  config:           Arc<Config>,
}

struct State {
  mspt: Arc<AtomicU32>,
}

impl World {
  pub fn new(
    block_converter: Arc<block::TypeConverter>,
    item_converter: Arc<item::TypeConverter>,
    entity_converter: Arc<entity::TypeConverter>,
    plugins: Arc<plugin::PluginManager>,
    commands: Arc<CommandTree>,
    wm: Arc<WorldManager>,
  ) -> Arc<Self> {
    let mut chunks = HashMap::new();
    let gen = if wm.config().get("world.use-schematic") {
      let path: &str = wm.config().get("world.schematic-path");
      schematic::load_from_file(&mut chunks, path)
        .unwrap_or_else(|err| error!("could not load schematic file {}: {}", path, err));
      WorldGen::new()
    } else {
      WorldGen::from_config(wm.config())
    };
    let world = Arc::new(World {
      chunks: RwLock::new(chunks),
      unloadable_chunks: Mutex::new(HashSet::new()),
      gen,
      players: RwLock::new(PlayersMap::new()),
      entities: RwLock::new(HashMap::new()),
      eid: 1.into(),
      block_converter,
      item_converter,
      entity_converter,
      plugins,
      commands,
      mspt: Arc::new(0.into()),
      wm,
      locked: true.into(),
    });
    let w = world.clone();
    thread::spawn(|| {
      w.init();
      w.global_tick_loop();
    });
    world
  }

  /// Returns the config used in the whole server.
  pub fn config(&self) -> &Arc<Config> { self.wm.config() }

  fn global_tick_loop(self: Arc<Self>) {
    let pool = ThreadPool::auto(|| State { mspt: self.mspt.clone() });
    let mut tick = 0;
    loop {
      let start = Instant::now();
      if tick % 20 == 0 {
        let mut header = Chat::empty();
        let mut footer = Chat::empty();

        header.add("big gaming\n").color(Color::Blue);
        footer.add("\nmspt: ");
        let mspt = self.mspt.swap(0, Ordering::SeqCst) / 20;
        footer.add(format!("{}", mspt)).color(if mspt > 50 {
          Color::Red
        } else if mspt > 20 {
          Color::Gold
        } else if mspt > 10 {
          Color::Yellow
        } else {
          Color::BrightGreen
        });

        let out = cb::Packet::PlayerHeader { header: header.to_json(), footer: footer.to_json() };
        for p in self.players().values() {
          p.send(out.clone());
        }
      }
      for p in self.players().iter() {
        let p = p.clone();
        pool.execute(move |s| {
          let start = Instant::now();
          // Updates the player correctly, and performs collision checks. This also
          // handles new chunks.
          p.tick();
          // Do player collision and packets and stuff
          // Once per second, send keep alive packet
          if tick % 20 == 0 {
            p.send(cb::Packet::KeepAlive { id: 1234556 });
          }
          s.mspt.fetch_add(start.elapsed().as_millis().try_into().unwrap(), Ordering::SeqCst);
        });
      }
      for (_eid, ent) in self.entities().iter() {
        let ent = ent.clone();
        pool.execute(move |s| {
          let start = Instant::now();
          ent.tick();
          s.mspt.fetch_add(start.elapsed().as_millis().try_into().unwrap(), Ordering::SeqCst);
        });
      }
      tick += 1;
      let time = Instant::now().duration_since(start);
      match Duration::from_millis(50).checked_sub(time) {
        Some(t) => thread::sleep(t),
        None => warn!("tick took more than 50 milliseconds: {}", time.as_millis()),
      }
    }
  }
  fn new_player(self: Arc<Self>, player: Player) -> Arc<Player> {
    let player = Arc::new(player);
    // We need to unlock players so that player_init() will work.
    {
      // If a bunch of people connect at the same time, we don't want a bunch of lock
      // contention.
      let players = self.players.read();
      if players.contains_key(&player.id()) {
        player.disconnect("Another player with the same id is already connected!");
        return player;
      }
      drop(players);
      let mut players = self.players.write();
      players.insert(player.id(), player.clone());
    }
    self.player_init(&player);
    player
  }

  /// Returns a new, unique EID.
  pub fn eid(&self) -> i32 { self.eid.fetch_add(1, Ordering::SeqCst) }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn block_converter(&self) -> &block::TypeConverter { &self.block_converter }
  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn item_converter(&self) -> &item::TypeConverter { &self.item_converter }
  /// Returns the current entity converter. This can be used to convert old
  /// entity ids to new ones, and vice versa.
  pub fn entity_converter(&self) -> &entity::TypeConverter { &self.entity_converter }
  /// Returns the plugin manager. This is how events can be sent to plugins.
  /// This is the same plugin manager returned by the [`WorldManager`], and by
  /// other worlds.
  pub fn plugins(&self) -> &plugin::PluginManager { &self.plugins }
  /// Returns the command tree that the server uses. This can be used to add
  /// custom commands to the server.
  pub fn commands(&self) -> &CommandTree { &self.commands }

  /// Returns the world manager for this world. This is a global value, used for
  /// things like what players are all online.
  pub fn world_manager(&self) -> &Arc<WorldManager> { &self.wm }

  /// Generates a chunk for the given chunk position. This will not store the
  /// chunk, or even look in the chunks table at all. It should be used if you
  /// have a list of chunks to generate, and you would like to generate them in
  /// parallel.
  pub fn pre_generate_chunk(&self, pos: ChunkPos) -> MultiChunk {
    let mut c = MultiChunk::new(self.block_converter.clone(), true);
    self.gen.generate(pos, &mut c);
    c
  }

  /// Checks if the given chunk position is loaded. This will not check for any
  /// data saved on disk, it only checks if the given chunk is in memory.
  pub fn has_loaded_chunk(&self, pos: ChunkPos) -> bool { self.chunks.read().contains_key(&pos) }

  /// Stores a list of chunks in the internal map. This should be used after
  /// calling [`pre_generate_chunk`](Self::pre_generate_chunk) a number of
  /// times.
  ///
  /// This will not overwrite any chunks that are already loaded. This is best
  /// for having another thread do terrain generation, then storing that terrain
  /// in the world. While that other thread was running, the world could have
  /// loaded something from disk, which you don't want to overwrite.
  pub fn store_chunks_no_overwrite(&self, chunks: Vec<(ChunkPos, MultiChunk)>) {
    // Only locks for reading if all the chunks are already in the world.
    let mut needs_write = false;
    {
      let read = self.chunks.read();
      for (pos, _) in &chunks {
        if !read.contains_key(pos) {
          needs_write = true;
          break;
        }
      }
    }
    if needs_write {
      let mut write = self.chunks.write();
      for (pos, c) in chunks {
        // Make sure to call or_insert_with. Someone could have changed the chunks
        // between the read unlock and the write lock. So the needs_write bool is mostly
        // an approximation.
        let ent = write.entry(pos).or_insert_with(|| CountedChunk::new(c));
        // If the chunk was already present, it might not have a count of 0.
        if ent.count.load(Ordering::Acquire) == 0 {
          self.unloadable_chunks.lock().insert(pos);
        }
      }
    }
  }

  /// This calls f(), and passes it a locked chunk.
  ///
  /// I tried to make the chunk a returned value, but that ended up being too
  /// difficult. Since the entire chunks map must be locked for reading, that
  /// read lock must be held while the chunk is in scope. Because of this, you
  /// would have needed to call two functions to get it working. I tried my best
  /// with the [`Deref`](std::ops::Deref) trait, but I couldn't get it to work
  /// the way I liked.
  pub fn chunk<F, R>(&self, pos: ChunkPos, f: F) -> R
  where
    F: FnOnce(MutexGuard<MultiChunk>) -> R,
  {
    // We first check (read-only) if we need to generate a new chunk
    if !self.chunks.read().contains_key(&pos) {
      // If we do, we lock it for writing
      let mut chunks = self.chunks.write();
      // Make sure that the chunk was not written in between locking this chunk
      let ent =
        chunks.entry(pos).or_insert_with(|| CountedChunk::new(self.pre_generate_chunk(pos)));
      // If the chunk was already present, it might not have a count of 0.
      if ent.count.load(Ordering::Acquire) == 0 {
        self.unloadable_chunks.lock().insert(pos);
      }
    }
    let chunks = self.chunks.read();
    let c = chunks[&pos].chunk.lock();
    f(c)
  }

  /// This serializes a chunk for the given version. This packet can be sent
  /// directly to a client. Note that on most vanilla versions, sending a chunk
  /// to a client that already has loaded that chunk will cause a memory leak.
  /// Unloading a chunk multiple times will not cause a memory leak. If you are
  /// trying to re-send an entire chunk to a player, make sure to send them an
  /// unload chunk packet first. Use at your own risk!
  ///
  /// If you are trying to produce a large block change packet, use
  /// [`serialize_partial_chunk`](Self::serialize_partial_chunk).
  pub fn serialize_chunk(&self, pos: ChunkPos) -> cb::Packet {
    self.chunk(pos, |c| {
      let mut bit_map = 0;
      let mut sections = vec![];
      let inner = c.inner();

      for (y, s) in inner.sections().enumerate() {
        if let Some(c) = s {
          bit_map |= 1 << y;
          sections.push(c.clone());
        }
      }

      cb::Packet::Chunk {
        pos,
        full: true,
        bit_map,
        sections,
        sky_light: c.sky_light().clone(),
        block_light: c.block_light().clone(),
      }
    })
  }

  /// This serializes a chunk for the given version. This packet can be sent
  /// directly to a client. Unlock [`serialize_chunk`](Self::serialize_chunk),
  /// this will not cause a memory leak. In fact, sending this in an unloaded
  /// chunk is undefined behavior! This should be used like a large multi block
  /// change packet.
  ///
  /// The `min` and `max` are section indices. These can be obtained through
  /// [`Pos::chunk_y`]. Every section between `min` and `max` (inclusive) will
  /// be sent to the client. If that second does not exist, this function will
  /// panic. `min` and `max` should not be outside of 0..15, unless you are
  /// sending this to a 1.17+ client.
  pub fn serialize_partial_chunk(&self, pos: ChunkPos, min: u32, max: u32) -> cb::Packet {
    self.chunk(pos, |c| {
      let mut bit_map = 0;
      let mut sections = vec![];
      let inner = c.inner();

      for (y, s) in inner.sections().enumerate() {
        if (y as u32) < min || y as u32 > max {
          continue;
        }
        if let Some(c) = s {
          bit_map |= 1 << y;
          sections.push(c.clone());
        }
      }

      cb::Packet::Chunk {
        pos,
        full: false,
        bit_map,
        sections,
        // TODO: Only clone the sections we care about
        sky_light: c.sky_light().clone(),
        block_light: c.block_light().clone(),
      }
    })
  }
  /// Serializes a multi block change packet. This is generally used in `/fill`
  /// commands, for chunks where only a few blocks have been changed.
  ///
  /// The iterator should contain a list of relative chunk positions, and block
  /// ids. This function will panic if any of these block positions are outside
  /// of the zero-zero chunk.
  #[track_caller]
  pub fn serialize_multi_block_change(
    &self,
    pos: ChunkPos,
    chunk_y: i32,
    changes: impl Iterator<Item = (Pos, u32)>,
  ) -> cb::Packet {
    cb::Packet::MultiBlockChange {
      pos,
      y: chunk_y,
      changes: changes
        .map(|(pos, id)| {
          if pos.x() < 0
            || pos.x() >= 16
            || pos.y() < 0
            || pos.y() >= 16
            || pos.z() < 0
            || pos.z() >= 16
          {
            panic!("invalid block position {}", pos);
          }
          (id as u64) << 12 | (pos.x() as u64) << 8 | (pos.y() as u64) << 4 | pos.z() as u64
        })
        .collect(),
    }
  }

  /// Increments how many people are viewing the given chunk. This counter is
  /// used to track when a chunk should be loaded/unloaded. This will load the
  /// given chunk if it is not loaded already.
  pub fn inc_view(&self, pos: ChunkPos) {
    // We first check (read-only) if we need to generate a new chunk
    if !self.chunks.read().contains_key(&pos) {
      // If we do, we lock it for writing
      let mut chunks = self.chunks.write();
      // Make sure that the chunk was not written in between locking this chunk
      chunks.entry(pos).or_insert_with(|| CountedChunk::new(self.pre_generate_chunk(pos)));
    }
    let chunks = self.chunks.read();
    let c = &chunks[&pos];
    // If the count was 0, the chunk might not have been present in
    // unloadable_chunks, as it might be the one we just added above. We know this
    // chunk should not be unloaded, so if an unloading task starts between adding
    // the chunk above and updating this value, we don't want the chunk to be in the
    // unloadable_chunks at all.
    if c.count.fetch_add(1, Ordering::Acquire) == 0 {
      self.unloadable_chunks.lock().remove(&pos);
    }
  }

  /// Decrements how many people are viewing the given chunk. This counter is
  /// used to track when a chunk should be loaded/unloaded. If this chunk does
  /// not exist, this will do nothing.
  pub fn dec_view(&self, pos: ChunkPos) {
    // We first check (read-only) if the chunk is present.
    if !self.chunks.read().contains_key(&pos) {
      return;
    }
    let chunks = self.chunks.read();
    let c = &chunks[&pos];
    // If the count was 1, then the chunk should be added to the list of chunks to
    // be unloaded. We don't unload it now, as we only want to lazily unload chunks.
    if c.count.fetch_sub(1, Ordering::Acquire) == 1 {
      self.unloadable_chunks.lock().insert(pos);
    }
  }

  /// This broadcasts a chat message to everybody in the world. Note that this
  /// does not lock the players map exclusively. So, if this is called twice,
  /// both operations will execute in parallel. This might cause some packets to
  /// arrive out of order between clients (one client would see one broadcast
  /// before the other). This is only possible if you call broadcast from
  /// multiple threads, as this blocks until all the packets are queued.
  pub fn broadcast(&self, msg: impl Into<Chat>) {
    let m = msg.into();
    for p in self.players.read().values() {
      p.send_message(&m);
    }
  }

  /// Returns a read lock on the players map.
  pub fn players(&self) -> RwLockReadGuard<'_, PlayersMap> { self.players.read() }

  /// Removes the given player from this world. This should be called from
  /// WorldManagger, so that the world managger's table of players to worlds
  /// stays synced.
  fn remove_player(&self, id: UUID) {
    let mut lock = self.players.write();
    let p = lock.remove(&id).unwrap();
    p.unload_all();
    if lock.is_empty() {
      drop(lock);
      self.unload_chunks();
      let len = self.chunks.read().len();
      if len != 0 {
        warn!("chunks remaining after last player logged off: {}", len);
      }
    }
  }

  // Unloads all the chunks that are cached for unloading.
  pub fn unload_chunks(&self) {
    let mut wl = self.chunks.write();
    for pos in self.unloadable_chunks.lock().drain() {
      wl.remove(&pos);
    }
  }

  /// Returns true if the world is locked. This is an atomic load, so it will
  /// always be a race condition. However, whenever you modify the world, this
  /// is also checked, so it won't end up being a problem.
  pub fn is_locked(&self) -> bool { self.locked.load(Ordering::Relaxed) }
}

impl Default for WorldManager {
  fn default() -> Self { WorldManager::new() }
}

impl WorldManager {
  pub fn new() -> Self {
    WorldManager {
      block_converter:  Arc::new(block::TypeConverter::new()),
      item_converter:   Arc::new(item::TypeConverter::new()),
      entity_converter: Arc::new(entity::TypeConverter::new()),
      plugins:          Arc::new(plugin::PluginManager::new()),
      commands:         Arc::new(CommandTree::new()),
      worlds:           Mutex::new(vec![]),
      players:          Mutex::new(HashMap::new()),
      config:           Arc::new(Config::new("config.yml", "default.yml")),
    }
  }

  /// Returns the config used in the whole server.
  pub fn config(&self) -> &Arc<Config> { &self.config }

  pub fn run(self: Arc<Self>) { self.plugins.clone().run(self); }

  /// Adds a new world.
  pub fn add_world(self: &Arc<Self>) {
    self.worlds.lock().push(World::new(
      self.block_converter.clone(),
      self.item_converter.clone(),
      self.entity_converter.clone(),
      self.plugins.clone(),
      self.commands.clone(),
      self.clone(),
    ));
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn block_converter(&self) -> &block::TypeConverter { &self.block_converter }

  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn item_converter(&self) -> &item::TypeConverter { &self.item_converter }
  /// Returns the plugins used for the whole server.
  pub fn plugins(&self) -> &plugin::PluginManager { &self.plugins }
  /// Returns the commands used for the whole server.
  pub fn commands(&self) -> &CommandTree { &self.commands }

  /// Broadcasts a message to everyone one the server.
  pub fn broadcast(&self, msg: impl Into<Chat>) {
    let m = msg.into();
    let worlds = self.worlds.lock();
    for w in worlds.iter() {
      for p in w.players.read().values() {
        p.send_message(&m);
      }
    }
  }

  /// Returns the default world. This can be used to easily get a world without
  /// any other context.
  pub fn default_world(&self) -> Arc<World> { self.worlds.lock()[0].clone() }

  // /// Adds a new player into the game. This should be called when a new grpc
  // /// proxy connects.
  // pub async fn new_player(&self, req: Streaming<Packet>, tx:
  // Sender<Result<Packet, Status>>) {   let mut conn = Connection::new(req,
  // tx);   let (username, uuid, ver) = conn.wait_for_login();
  //   let w = self.worlds.lock()[0].clone();
  //   let player = Player::new(
  //     w.eid(),
  //     username,
  //     uuid,
  //     Arc::new(conn),
  //     ver,
  //     w.clone(),
  //     FPos::new(0.0, 60.0, 0.0),
  //   );
  //   w.new_player(player);
  // }
  /// Adds a new player into the game. This should be called when a new grpc
  /// proxy connects.
  pub fn new_player(
    &self,
    conn: ConnSender,
    username: String,
    uuid: UUID,
    ver: ProtocolVersion,
  ) -> Arc<Player> {
    let w = self.worlds.lock()[0].clone();
    let player =
      Player::new(w.eid(), username, uuid, conn, ver, w.clone(), FPos::new(0.0, 150.0, 0.0));
    self.players.lock().insert(uuid, 0);
    w.new_player(player)
  }

  /// Removes the player. This is not part of the public API because it does not
  /// terminate their connection. This is called after their connection is
  /// terminated.
  pub(crate) fn remove_player(&self, id: UUID) {
    let idx = *self.players.lock().get(&id).unwrap();
    self.worlds.lock()[idx].remove_player(id);
    self.players.lock().remove(&id);
  }
}
