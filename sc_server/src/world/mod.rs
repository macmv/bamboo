mod blocks;
pub mod chunk;
pub mod gen;
mod init;
mod players;

use parking_lot::{Mutex, MutexGuard, RwLock};
use std::{
  collections::HashMap,
  convert::TryInto,
  sync::{
    atomic::{AtomicI32, AtomicU32, Ordering},
    Arc,
  },
  thread,
  thread::ThreadId,
  time::{Duration, Instant},
};

use sc_common::{
  math::{ChunkPos, FPos},
  net::cb,
  util::{
    chat::{Chat, Color},
    ThreadPool, UUID,
  },
  version::{BlockVersion, ProtocolVersion},
};

use crate::{block, command::CommandTree, entity, item, net::ConnSender, player::Player, plugin};
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

pub struct World {
  chunks:           RwLock<HashMap<ChunkPos, Arc<Mutex<MultiChunk>>>>,
  generators:       RwLock<HashMap<ThreadId, Mutex<WorldGen>>>,
  players:          Mutex<PlayersMap>,
  eid:              AtomicI32,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
  commands:         Arc<CommandTree>,
  mspt:             Arc<AtomicU32>,
  wm:               Arc<WorldManager>,
}

pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds:           Mutex<Vec<Arc<World>>>,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
  commands:         Arc<CommandTree>,
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
    let world = Arc::new(World {
      chunks: RwLock::new(HashMap::new()),
      generators: RwLock::new(HashMap::new()),
      players: Mutex::new(PlayersMap::new()),
      eid: 1.into(),
      block_converter,
      item_converter,
      entity_converter,
      plugins,
      commands,
      mspt: Arc::new(0.into()),
      wm,
    });
    let w = world.clone();
    thread::spawn(|| {
      w.init();
      w.global_tick_loop();
    });
    world
  }
  fn global_tick_loop(self: Arc<Self>) {
    let pool = ThreadPool::auto();
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

        let out =
          cb::Packet::PlayerlistHeader { header: header.to_json(), footer: footer.to_json() };
        for p in self.players().values() {
          p.send(out.clone());
        }
      }
      for p in self.players().iter() {
        let wm = &self.clone();
        let mspt = self.mspt.clone();
        let p = p.clone();
        pool.execute(move || {
          let start = Instant::now();
          // Updates the player correctly, and performs collision checks. This also
          // handles new chunks.
          p.tick();
          // Do player collision and packets and stuff
          // Once per second, send keep alive packet
          if tick % 20 == 0 {
            p.send(cb::Packet::KeepAlive {
              keep_alive_id_v1_8:    Some(1234556),
              keep_alive_id_v1_12_2: Some(1234556),
            });
          }
          mspt.fetch_add(start.elapsed().as_millis().try_into().unwrap(), Ordering::SeqCst);
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
      let mut players = self.players.lock();
      if players.contains_key(&player.id()) {
        player.disconnect("Another player with the same id is already connected!");
        return player;
      }
      players.insert(player.id(), player.clone());
    }
    self.player_init(&player);
    player
  }

  /// Returns a new, unique EID.
  pub fn eid(&self) -> i32 {
    self.eid.fetch_add(1, Ordering::SeqCst)
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn block_converter(&self) -> &block::TypeConverter {
    &self.block_converter
  }
  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn item_converter(&self) -> &item::TypeConverter {
    &self.item_converter
  }
  /// Returns the current entity converter. This can be used to convert old
  /// entity ids to new ones, and vice versa.
  pub fn entity_converter(&self) -> &entity::TypeConverter {
    &self.entity_converter
  }
  /// Returns the plugin manager. This is how events can be sent to plugins.
  /// This is the same plugin manager returned by the [`WorldManager`], and by
  /// other worlds.
  pub fn plugins(&self) -> &plugin::PluginManager {
    &self.plugins
  }
  /// Returns the command tree that the server uses. This can be used to add
  /// custom commands to the server.
  pub fn commands(&self) -> &CommandTree {
    &self.commands
  }

  /// Generates a chunk for the given chunk position. This will not store the
  /// chunk, or even look in the chunks table at all. It should be used if you
  /// have a list of chunks to generate, and you would like to generate them in
  /// parallel.
  pub fn pre_generate_chunk(&self, pos: ChunkPos) -> MultiChunk {
    let tid = thread::current().id();
    // We first check (read-only) if we need a world generator for this thread
    if !self.generators.read().contains_key(&tid) {
      // If we do, we lock it for writing
      let mut generators = self.generators.write();
      // Make sure that the chunk was not written in between locking this chunk
      // Even though we only use this generator on this thread, Rust safety says we
      // need a Mutex here. I could do away with the mutex in unsafe code, but that
      // seems like a pre-mature optimization.
      generators.entry(tid).or_insert_with(|| Mutex::new(WorldGen::new()));
    }
    let generators = self.generators.read();
    let mut lock = generators[&tid].lock();
    let mut c = MultiChunk::new(self.block_converter.clone());
    lock.generate(pos, &mut c);
    c
  }

  /// Checks if the given chunk position is loaded. This will not check for any
  /// data saved on disk, it only checks if the given chunk is in memory.
  pub fn has_loaded_chunk(&self, pos: ChunkPos) -> bool {
    self.chunks.read().contains_key(&pos)
  }

  /// Stores a list of chunks in the internal map. This should be used if you
  /// have manually built a chunk, and need to store it in the world. This
  /// should not be used after calling `pre_generate_chunk`, as the world may
  /// have loaded something from disk since that call. See also
  /// [`store_chunks_no_overwrite`](Self::store_chunks_no_overwrite).
  ///
  /// WARNING: This will override pre-existing chunks! This should not be a
  /// problem with multiple threads generating the same chunks, as they have
  /// already done most of the work by the time the override check occurs.
  pub fn store_chunks(&self, chunks: Vec<(ChunkPos, MultiChunk)>) {
    let mut lock = self.chunks.write();
    for (pos, c) in chunks {
      lock.insert(pos, Arc::new(Mutex::new(c)));
    }
  }

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
        write.entry(pos).or_insert_with(|| Arc::new(Mutex::new(c)));
      }
    }
  }

  /// This calls f(), and passes it a locked chunk. This will also generate a
  /// new chunk if there is not one stored there.
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
      chunks.entry(pos).or_insert_with(|| Arc::new(Mutex::new(self.pre_generate_chunk(pos))));
    }
    let chunks = self.chunks.read();
    let c = chunks[&pos].lock();
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
  pub fn serialize_chunk(&self, pos: ChunkPos, ver: BlockVersion) -> cb::Packet {
    self.chunk(pos, |c| crate::net::serialize::serialize_chunk(pos, &c, ver))
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
  pub fn serialize_partial_chunk(
    &self,
    pos: ChunkPos,
    ver: BlockVersion,
    min: u32,
    max: u32,
  ) -> cb::Packet {
    self.chunk(pos, |c| crate::net::serialize::serialize_partial_chunk(pos, &c, ver, min, max))
  }

  /// This broadcasts a chat message to everybody in the world.
  pub fn broadcast<M: Into<Chat>>(&self, msg: M) {
    let out = cb::Packet::Chat {
      message:      msg.into().to_json(),
      position:     0, // Chat box, not above hotbar
      sender_v1_16: Some(UUID::from_u128(0)),
    };

    for p in self.players.lock().values() {
      p.send(out.clone());
    }
  }

  // Runs f for all players within render distance of the chunk.
  pub fn players(&self) -> MutexGuard<'_, PlayersMap> {
    self.players.lock()
  }
}

impl Default for WorldManager {
  fn default() -> Self {
    WorldManager::new()
  }
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
    }
  }

  pub fn run(self: Arc<Self>) {
    self.plugins.clone().run(self);
  }

  /// Adds a new world. Currently, this requires a mutable reference, which
  /// cannot be obtained outside of initialization.
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
  pub fn block_converter(&self) -> &block::TypeConverter {
    &self.block_converter
  }

  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn item_converter(&self) -> &item::TypeConverter {
    &self.item_converter
  }
  /// Returns the plugins used for the whole server.
  pub fn plugins(&self) -> &plugin::PluginManager {
    &self.plugins
  }
  /// Returns the commands used for the whole server.
  pub fn commands(&self) -> &CommandTree {
    &self.commands
  }

  /// Broadcasts a message to everyone one the server.
  pub fn broadcast<M: Into<Chat>>(&self, msg: M) {
    let out = cb::Packet::Chat {
      message:      msg.into().to_json(),
      position:     0, // Chat box, not above hotbar
      sender_v1_16: Some(UUID::from_u128(0)),
    };

    let worlds = self.worlds.lock();
    for w in worlds.iter() {
      for p in w.players.lock().values() {
        p.send(out.clone());
      }
    }
  }

  /// Returns the default world. This can be used to easily get a world without
  /// any other context.
  pub fn default_world(&self) -> Arc<World> {
    self.worlds.lock()[0].clone()
  }

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
      Player::new(w.eid(), username, uuid, conn, ver, w.clone(), FPos::new(0.0, 60.0, 0.0));
    w.new_player(player)
  }
}
