//! Handles all the worlds on a Bamboo server.
//!
//! This has two main types, [`World`] and [`WorldManager`].
//!
//! [`World`] handles everything for a single world. This includes chunks, world
//! tick loops, players, and entities.
//!
//! [`WorldManager`] handles everything for the whole server. It is sort of the
//! global Bamboo type. There is one `WorldManager` per server. This handles
//! global things, like all the teams, players, and worlds. It also handles new
//! players joining, and players leaving. Lastly, it also contains a global tick
//! loop, which is currently only used for plugins.

mod bbr;
mod blocks;
mod chunk;
mod chunks;
mod entities;
pub mod gen;
mod init;
mod players;
mod region;
pub mod schematic;

use bb_common::{
  config::{Config, ConfigSection},
  math::{ChunkPos, FPos, Pos, SectionRelPos},
  net::cb,
  util::{
    chat::{Chat, Color},
    GameMode, JoinInfo, ThreadPool, UUID,
  },
};
use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard};
use std::{
  collections::HashMap,
  convert::TryInto,
  fmt,
  sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
    Arc,
  },
  thread,
  time::{Duration, Instant},
};

use crate::{
  block,
  command::CommandTree,
  data::Data,
  entity,
  entity::Entity,
  event, item,
  net::ConnSender,
  particle::Particle,
  player::{Player, Team},
  plugin,
  tags::Tags,
};

pub use chunk::{CountedChunk, MultiChunk};
pub use entities::{EntitiesIter, EntitiesMap, EntitiesMapRef};
pub use players::{PlayersIter, PlayersMap};

use bbr::{RegionMap, RegionRelPos};
use chunks::ChunksToLoad;
use gen::WorldGen;

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

/// A Minecraft world. This has a list of players, a collection of chunks, and a
/// collection of entities.
///
/// This also contains a bunch of references to other server stuff, such as
/// [block]/[item]/[entity] type converters, and the [`WorldManager`].
pub struct World {
  regions:          RegionMap,
  // generator:        String,
  gen:              WorldGen,
  players:          RwLock<PlayersMap>,
  entities:         RwLock<EntitiesMap>,
  eid:              AtomicI32,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
  commands:         Arc<CommandTree>,
  uspt:             Arc<AtomicU32>,
  wm:               Arc<WorldManager>,
  config:           ConfigSection,
  // If set, then the world cannot be modified.
  locked:           AtomicBool,

  chunks_to_load: Mutex<ChunksToLoad>,

  /// A height in blocks. Default is `256`.
  height: u32,
  /// A height in blocks. Default is `0`.
  min_y:  i32,
}

/// The world manager. This is essentially a Bamboo type. It stores all the
/// global state for the server.
///
/// This has a list of worlds, and it knows about every online player. This is
/// also where the [block]/[item]/[entity] type converters are created.
pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds:           RwLock<Vec<Arc<World>>>,
  // Player id to world index and player
  players:          RwLock<HashMap<UUID, (usize, Arc<Player>)>>,
  // Team name to team
  teams:            RwLock<HashMap<String, Arc<Mutex<Team>>>>,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
  tags:             Arc<Tags>,
  commands:         Arc<CommandTree>,
  config:           Arc<Config>,
  block_behaviors:  RwLock<block::BehaviorStore>,
  item_behaviors:   RwLock<item::BehaviorStore>,
  data:             Arc<Data>,

  default_game_mode: GameMode,
  spawn_point:       FPos,
}

struct State {
  uspt:  Arc<AtomicU32>,
  world: Arc<World>,
}

const TICK_TIME: Duration = Duration::from_millis(50);

impl World {
  /// Creates a new world. See also [`WorldManager::add_world`].
  pub(crate) fn new(
    block_converter: Arc<block::TypeConverter>,
    item_converter: Arc<item::TypeConverter>,
    entity_converter: Arc<entity::TypeConverter>,
    plugins: Arc<plugin::PluginManager>,
    commands: Arc<CommandTree>,
    wm: Arc<WorldManager>,
  ) -> Arc<Self> {
    let config = wm.config().section("world");
    let gen = WorldGen::from_config(&config);
    /*
    for schematic in config.get::<_, Vec<String>>("schematics") {
      let path = schematic.get("path");
      let pos = Pos::new(schematic.get("x"), schematic.get("y"), schematic.get("z"));
      schematic::load_from_file(&mut chunks, path, &block_converter, || {
        CountedChunk::new(MultiChunk::new(block_converter.clone(), true))
      })
      .unwrap_or_else(|err| error!("could not load schematic file {}: {}", path, err));
    }
    */
    let world = Arc::new_cyclic(|weak| World {
      regions: RegionMap::new(weak.clone(), config.get("save")),
      // generator: config.get("generator"),
      gen,
      players: RwLock::new(PlayersMap::new()),
      entities: RwLock::new(EntitiesMap::new()),
      // All player's think they are EID 1, so we start at 2. EID 0 is invalid.
      eid: 2.into(),
      block_converter,
      item_converter,
      entity_converter,
      plugins,
      commands,
      uspt: Arc::new(0.into()),
      locked: config.get::<bool>("locked").into(),
      height: config.get("height"),
      min_y: config.get("min_y"),
      config,
      wm,
      chunks_to_load: Mutex::new(ChunksToLoad::new()),
    });
    let vanilla = world.config().section("vanilla");
    if vanilla.get("enabled") {
      world.load_from_disk(&std::path::PathBuf::new().join(vanilla.get::<&str>("path"))).unwrap();
    }
    // Note that the world is not initialized yet, as we want to load plugins before
    // initializing.
    world
  }

  /// Returns the config used in the whole server.
  pub fn config(&self) -> &ConfigSection { &self.config }

  fn global_tick_loop(self: Arc<Self>) {
    let pool = ThreadPool::auto("global tick loop", || State {
      uspt:  self.uspt.clone(),
      world: Arc::clone(&self),
    });
    // We set a limit to double the number of cores. This means that we will only
    // hit an artificial limit if we can generate a chunk in 10 ms. The more we
    // increase this, the worse the ordering for generating chunks gets.
    //
    // TODO: Make this configurable.
    let chunk_pool =
      ThreadPool::auto_with_limit("chunk generator", bb_common::util::num_cpus() * 5, || State {
        uspt:  self.uspt.clone(),
        world: Arc::clone(&self),
      });
    let mut tick = 0;
    let mut start = Instant::now();
    let mut needs_to_unload = false;
    loop {
      if tick % 20 == 0 {
        let mut header = Chat::empty();
        let mut footer = Chat::empty();

        header.add("big gaming\n").color(Color::Blue);
        footer.add("\nuspt: ");
        let uspt = self.uspt.swap(0, Ordering::SeqCst) / 20;
        footer.add(uspt.to_string()).color(if uspt > 50_000 {
          Color::Red
        } else if uspt > 20_000 {
          Color::Gold
        } else if uspt > 10_000 {
          Color::Yellow
        } else {
          Color::BrightGreen
        });

        let out = cb::packet::PlayerHeader { header: header.to_json(), footer: footer.to_json() };
        for p in self.players().values() {
          p.send(out.clone());
        }
      }
      // Every 30 seconds, try to unload chunks we don't need. We do this on another
      // thread, as unloading chunks is expensive. If the chunk pool is full, we just
      // try again next tick. We do all of this before `check_chunks_queue`, as this
      // might also push tasks to `chunk_pool`, and unloading chunks should have
      // higher priority than loading more chunks.
      if tick % (20 * 30) == 0 {
        needs_to_unload = true;
      }
      if needs_to_unload {
        let res = chunk_pool.try_execute(|s| {
          s.world.unload_chunks();
        });
        if res.is_ok() {
          needs_to_unload = false
        }
      }
      self.check_chunks_queue(&chunk_pool);
      /*
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
            p.send(cb::packet::KeepAlive { id: 1234556 });
          }
          s.uspt.fetch_add(start.elapsed().as_micros().try_into().unwrap(), Ordering::SeqCst);
        });
      }
      */
      for (&eid, ent) in self.entities().iter_values() {
        let ent = ent.clone();
        let w = self.clone();
        pool.execute(move |s| {
          if let Some(ent) = ent.as_entity_ref(w.as_ref()) {
            let start = Instant::now();
            if ent.tick() {
              s.world.entities.write().remove(&eid);
              for p in s.world.players().iter().in_view(ent.pos().block().chunk()) {
                p.send(cb::packet::RemoveEntities { eids: vec![eid] });
              }
            }
            s.uspt.fetch_add(start.elapsed().as_micros().try_into().unwrap(), Ordering::SeqCst);
          }
        });
      }
      // We don't want overlapping tick loops
      pool.wait();
      tick += 1;
      let passed = Instant::now().duration_since(start);
      start += TICK_TIME;
      match TICK_TIME.checked_sub(passed) {
        Some(t) => spin_sleep::sleep(t),
        None => warn!("tick took {passed:?} (more than 50 ms)"),
      }
    }
  }
  fn new_player(self: Arc<Self>, player: Arc<Player>, info: JoinInfo) {
    {
      // let mut meta = bb_common::metadata::Metadata::new();
      // meta.set_item(8, item::Stack::new(item::Type::DebugStick).to_item());
      // self.summon_meta(entity::Type::Item, player.pos(), meta);
    }
    // We need to unlock players so that player_init() will work.
    {
      // If a bunch of people connect at the same time, we don't want a bunch of lock
      // contention.
      let players = self.players.read();
      if let Some(existing_player) = players.get(player.id()) {
        warn!(
          "a player named {} tried to join, but had the same id as {} (id: {:?})",
          player.username(),
          existing_player.username(),
          player.id(),
        );
        player.disconnect("Another player with the same id is already connected!");
        return;
      }
      drop(players);
      let mut players = self.players.write();
      players.insert(player.id(), player.clone());
      let mut entities = self.entities.write();
      entities.insert(player.eid(), Entity::Player(player.id()));
    }
    info!("{} has joined the game", player.username());

    if self.world_manager().config().get("join-messages") {
      // TODO: This message's format should be configurable
      let mut msg = Chat::empty();
      msg.add(player.username()).color(Color::BrightGreen);
      msg.add(" has joined").color(Color::Gray);
      self.world_manager().broadcast(msg);
    }

    self.player_init(&player, info);
    // We want our plugin stuff to trigger after the player has received all the
    // chunks and whatever other initialization stuff. This means we can't screw
    // anything up with the loading process (like trying to teleport the player).
    self.events().player_event(event::PlayerJoin { player });
  }

  /// Returns a new, unique EID.
  pub fn new_eid(&self) -> i32 { self.eid.fetch_add(1, Ordering::SeqCst) }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn block_converter(&self) -> &Arc<block::TypeConverter> { &self.block_converter }
  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn item_converter(&self) -> &Arc<item::TypeConverter> { &self.item_converter }
  /// Returns the current entity converter. This can be used to convert old
  /// entity ids to new ones, and vice versa.
  pub fn entity_converter(&self) -> &Arc<entity::TypeConverter> { &self.entity_converter }
  /// Returns the plugin manager. This is how events can be sent to plugins.
  /// This is the same plugin manager returned by the [`WorldManager`], and by
  /// other worlds.
  pub fn plugins(&self) -> &Arc<plugin::PluginManager> { &self.plugins }
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
    let mut c = MultiChunk::new(self.world_manager().clone(), true, self.height, self.min_y);
    self.gen.generate(pos, &mut c);
    c
    /*
    let mut c = Arc::new(Mutex::new(MultiChunk::new(self.world_manager().clone(), true)));
    self.plugins.on_generate_chunk(&self.generator, c.clone(), pos);
    loop {
      // self.gen.generate(pos, &mut c);
      c = match Arc::try_unwrap(c) {
        Ok(c) => return c.into_inner(),
        Err(c) => {
          std::thread::yield_now();
          c
        }
      }
    }
    */
  }

  /// Checks if the given chunk position is loaded. This will not check for any
  /// data saved on disk, it only checks if the given chunk is in memory.
  pub fn has_loaded_chunk(&self, pos: ChunkPos) -> bool { self.regions.has_chunk(pos) }

  /// Stores a list of chunks in the internal map. This should be used after
  /// calling [`pre_generate_chunk`](Self::pre_generate_chunk) a number of
  /// times.
  ///
  /// This will not overwrite any chunks that are already loaded. This is best
  /// for having another thread do terrain generation, then storing that terrain
  /// in the world. While that other thread was running, the world could have
  /// loaded something from disk, which you don't want to overwrite.
  pub fn store_chunks_no_overwrite(&self, chunks: Vec<(ChunkPos, MultiChunk)>) {
    for (pos, chunk) in chunks {
      self.regions.region(pos, |mut region| {
        region.get_or_generate(pos, || CountedChunk::new(chunk));
      });
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
    self.regions.region(pos, |mut region| {
      let chunk = region.get_or_generate(RegionRelPos::new(pos), || {
        CountedChunk::new(self.pre_generate_chunk(pos))
      });
      f(chunk.lock())
    })
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
  pub fn serialize_chunk(&self, pos: ChunkPos) -> cb::packet::Chunk {
    self.chunk(pos, |c| {
      let inner = c.inner();

      let mut sections: Vec<_> = inner.sections().cloned().collect();
      sections.resize((self.height as usize + 15) / 16, None);
      cb::packet::Chunk {
        pos,
        full: true,
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
  pub fn serialize_partial_chunk(&self, pos: ChunkPos, min: u32, max: u32) -> cb::packet::Chunk {
    self.chunk(pos, |c| {
      let inner = c.inner();

      let mut sections: Vec<_> = inner
        .sections()
        .enumerate()
        .map(|(y, s)| if (y as u32) < min || y as u32 > max { None } else { s.clone() })
        .collect();
      sections.resize((self.height as usize + 15) / 16, None);
      cb::packet::Chunk {
        pos,
        full: false,
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
    changes: impl Iterator<Item = (SectionRelPos, u32)>,
  ) -> cb::packet::MultiBlockChange {
    cb::packet::MultiBlockChange {
      pos,
      y: chunk_y,
      changes: changes
        .map(|(pos, id)| {
          (id as u64) << 12 | (pos.x() as u64) << 8 | (pos.y() as u64) << 4 | pos.z() as u64
        })
        .collect(),
    }
  }

  /// Increments how many people are viewing the given chunk. This counter is
  /// used to track when a chunk should be loaded/unloaded. This will load the
  /// chunk if it is not already present.
  pub fn inc_view(&self, pos: ChunkPos) {
    self.regions.region(pos, |mut region| {
      let chunk = region.get_or_generate(RegionRelPos::new(pos), || {
        CountedChunk::new(self.pre_generate_chunk(pos))
      });
      chunk.count.fetch_add(1, Ordering::SeqCst);
    })
  }

  /// Decrements how many people are viewing the given chunk. This counter is
  /// used to track when a chunk should be loaded/unloaded. If this chunk does
  /// not exist, this will do nothing.
  pub fn dec_view(&self, pos: ChunkPos) {
    self.regions.region(pos, |mut region| {
      let chunk = region.get_or_generate(RegionRelPos::new(pos), || {
        CountedChunk::new(self.pre_generate_chunk(pos))
      });
      chunk.count.fetch_sub(1, Ordering::SeqCst);
    })
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
      p.send_message(m.clone());
    }
  }

  /// Returns a read lock on the players map.
  pub fn players(&self) -> RwLockReadGuard<'_, PlayersMap> { self.players.read() }

  /// Removes the given player from this world. This should be called from
  /// WorldManagger, so that the world managger's table of players to worlds
  /// stays synced.
  fn remove_player(&self, id: UUID) {
    let mut lock = self.players.write();
    // If the player is not present, this player has already been removed.
    if let Some(p) = lock.remove(&id) {
      let players_is_empty = lock.is_empty();
      drop(lock);

      self.entities.write().remove(&p.eid());
      self.events().player_event(event::PlayerLeave { player: p.clone() });
      info!("{} left the game", p.username());

      if self.world_manager().config().get("leave-messages") {
        // TODO: This message's format should be configurable
        let mut msg = Chat::empty();
        msg.add(p.username()).color(Color::BrightGreen);
        msg.add(" has left").color(Color::Gray);
        self.world_manager().broadcast(msg);
      }

      let entity_remove = cb::packet::RemoveEntities { eids: vec![p.eid()] };
      let list_remove = cb::packet::PlayerList {
        action: cb::PlayerListAction::Remove(vec![cb::PlayerListRemove { id: p.id() }]),
      };
      for other in p.world().players().iter().in_view(p.pos().block().chunk()).not(p.id()) {
        other.send(entity_remove.clone());
        other.send(list_remove.clone());
      }
      p.unload_all();

      if players_is_empty {
        self.unload_chunks();
        /*
        let len = self.chunks.read().len();
        if len != 0 {
          warn!("chunks remaining after last player logged off: {}", len);
        }
        */
      }
    }
  }

  // Unloads all the chunks that are cached for unloading.
  pub fn unload_chunks(&self) { self.regions.unload_chunks(); }

  /// Returns true if the world is locked. This is an atomic load, so it will
  /// always be a race condition. However, whenever you modify the world, this
  /// is also checked, so it won't end up being a problem.
  pub fn is_locked(&self) -> bool { self.locked.load(Ordering::Relaxed) }

  /// Plays the given sound at the given positions. All nearby players will be
  /// able to hear it.
  pub fn play_sound(
    &self,
    sound: String,
    category: cb::SoundCategory,
    pos: FPos,
    volume: f32,
    pitch: f32,
  ) {
    let out = cb::packet::PlaySound { name: sound, category, pos, volume, pitch };
    for p in self.players().iter().in_view(pos.block().chunk()) {
      p.send(out.clone());
    }
  }

  pub fn spawn_particle(&self, particle: Particle) {
    for p in self.players().iter().in_view(particle.pos.chunk()) {
      p.send_particle(particle.clone());
    }
  }

  /// Searches upwards for an open spawn point, based on the `start` position.
  /// This may return a position outside the world.
  pub fn find_spawn_point(&self, start: Pos) -> Pos {
    let mut pos = start;
    loop {
      let lo = pos;
      let hi = pos + Pos::new(0, 1, 0);
      if self.get_kind(lo).map(|k| k == block::Kind::Air).unwrap_or(true)
        && self.get_kind(hi).map(|k| k == block::Kind::Air).unwrap_or(true)
      {
        break pos;
      }
      pos += Pos::new(0, 1, 0);
    }
  }

  pub fn save(&self) { self.regions.save(); }
}

impl fmt::Debug for WorldManager {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("WorldManager").field("players", &self.players.read().len()).finish()
  }
}

impl WorldManager {
  pub fn new(write_default: bool) -> Self {
    let config = if write_default {
      Config::new_write_default(
        "server.toml",
        "server-default.toml",
        include_str!("../default.toml"),
      )
    } else {
      Config::new("server.toml", include_str!("../default.toml"))
    };
    WorldManager::new_with_config(config)
  }

  pub fn new_with_config(config: Config) -> Self {
    WorldManager {
      block_converter:   Arc::new(block::TypeConverter::new()),
      item_converter:    Arc::new(item::TypeConverter::new()),
      entity_converter:  Arc::new(entity::TypeConverter::new()),
      plugins:           Arc::new(plugin::PluginManager::new()),
      commands:          Arc::new(CommandTree::new()),
      tags:              Arc::new(Tags::new()),
      block_behaviors:   RwLock::new(block::BehaviorStore::new()),
      item_behaviors:    RwLock::new(item::BehaviorStore::new()),
      data:              Arc::new(Data::load(config.get("data-path"))),
      worlds:            RwLock::new(vec![]),
      players:           RwLock::new(HashMap::new()),
      teams:             RwLock::new(HashMap::new()),
      default_game_mode: config.get("default-gamemode"),
      spawn_point:       config.get("spawn-point"),
      config:            Arc::new(config),
    }
  }

  /// Returns a list of all worlds on the server.
  pub fn worlds(&self) -> RwLockReadGuard<'_, Vec<Arc<World>>> { self.worlds.read() }

  /// Returns the default game mode for the server. Used when a new client
  /// connects.
  pub fn default_game_mode(&self) -> GameMode { self.default_game_mode }

  /// Returns a list of all the teams on the server.
  pub fn teams(&self) -> RwLockReadGuard<'_, HashMap<String, Arc<Mutex<Team>>>> {
    self.teams.read()
  }

  /// Loads plugins
  pub fn load_plugins(self: &Arc<Self>) { self.plugins.load(self.clone()) }

  /// Returns the config used in the whole server.
  pub fn config(&self) -> &Arc<Config> { &self.config }

  /// Runs a global tick loop. This is used for plugin events. This is a
  /// blocking call.
  pub fn run(self: Arc<Self>) { self.global_tick_loop(); }

  /// Adds a new world.
  pub fn add_world(self: &Arc<Self>) -> Arc<World> {
    let world = World::new(
      self.block_converter.clone(),
      self.item_converter.clone(),
      self.entity_converter.clone(),
      self.plugins.clone(),
      self.commands.clone(),
      self.clone(),
    );
    let w = Arc::clone(&world);
    thread::spawn(move || {
      w.global_tick_loop();
    });
    let w2 = Arc::clone(&world);
    self.worlds.write().push(world);
    w2
  }
  #[cfg(test)]
  pub(crate) fn add_world_no_tick(self: &Arc<Self>) {
    self.worlds.write().push(World::new(
      self.block_converter.clone(),
      self.item_converter.clone(),
      self.entity_converter.clone(),
      self.plugins.clone(),
      self.commands.clone(),
      self.clone(),
    ));
  }
  /// Creates a team. Returns `None`, and does nothing, if there is already a
  /// team with the given name.
  pub fn create_team(self: &Arc<Self>, name: String) -> Option<Arc<Mutex<Team>>> {
    let mut wl = self.teams.write();
    if wl.contains_key(&name) {
      return None;
    }
    let team = Arc::new(Mutex::new(Team::new(self.clone(), name.clone())));
    wl.insert(name, team.clone());
    Some(team)
  }
  /// Gets the team with the given name. If it doesn't exist, this will return
  /// `None`.
  pub fn team(&self, name: &str) -> Option<Arc<Mutex<Team>>> {
    let rl = self.teams.read();
    rl.get(name).cloned()
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn block_converter(&self) -> &Arc<block::TypeConverter> { &self.block_converter }

  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn item_converter(&self) -> &Arc<item::TypeConverter> { &self.item_converter }
  /// Returns the current entity converter. This can be used to convert old
  /// entity ids to new ones, and vice versa. It can also be used for
  /// converting entity metadata indices.
  pub fn entity_converter(&self) -> &Arc<entity::TypeConverter> { &self.entity_converter }
  /// Returns the plugins used for the whole server.
  pub fn plugins(&self) -> &Arc<plugin::PluginManager> { &self.plugins }
  /// Returns the commands used for the whole server.
  pub fn commands(&self) -> &CommandTree { &self.commands }

  /// Returns a read lock on the block behavior storage.
  pub fn block_behaviors(&self) -> RwLockReadGuard<'_, block::BehaviorStore> {
    self.block_behaviors.read()
  }

  /// Returns a read lock on the item behavior storage.
  pub fn item_behaviors(&self) -> RwLockReadGuard<'_, item::BehaviorStore> {
    self.item_behaviors.read()
  }

  /// Returns the json data. This will include crafting recipes, and more data
  /// in the future.
  pub fn json_data(&self) -> &Arc<Data> { &self.data }

  /// Returns the tags for this server. This is mostly used for serializing
  /// packets. If you need the tags on a specific item/block, use `get` on
  /// [`block_converter`](Self::block_converter) or
  /// [`item_converter`](Self::item_converter).
  pub fn tags(&self) -> &Tags { &self.tags }

  /// Broadcasts a message to everyone one the server.
  ///
  /// # Example
  /// ```
  /// # use bb_server::world::WorldManager;
  /// # let wm = WorldManager::new(false);
  /// wm.broadcast("Hello world!");
  /// ```
  pub fn broadcast(&self, msg: impl Into<Chat>) {
    let m = msg.into();
    let worlds = self.worlds.read();
    for w in worlds.iter() {
      for p in w.players.read().values() {
        p.send_message(m.clone());
      }
    }
  }

  /// Returns the default world. This can be used to easily get a world without
  /// any other context.
  pub fn default_world(&self) -> Arc<World> { self.worlds.read()[0].clone() }

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
  pub fn new_player(&self, conn: ConnSender, info: JoinInfo) -> Arc<Player> {
    let w = self.worlds.read()[0].clone();
    let spawn = if self.config().get("find-spawn") {
      w.find_spawn_point(self.spawn_point.block()).into()
    } else {
      self.spawn_point
    };
    let player = Player::new(w.new_eid(), conn, info.clone(), w.clone(), spawn);
    self.players.write().insert(info.uuid, (0, player.clone()));
    w.new_player(player.clone(), info);
    player
  }

  /// Removes the player. This is not part of the public API because it does not
  /// terminate their connection. This is called after their connection is
  /// terminated.
  ///
  /// If the player is not present, this will do nothing.
  pub(crate) fn remove_player(&self, id: UUID) {
    let idx = match self.players.read().get(&id) {
      Some(v) => v.0,
      None => return,
    };
    // This must be a read lock, or else this deadlocks (because of the leave
    // message broadcast).
    self.worlds.read()[idx].remove_player(id);
    // Avoid race condition, this needs to be before we remove `id` from `players`.
    // If we do this after, then the team might iterate through its own players and
    // need to look them up from `self.players`.
    for (_, team) in self.teams.read().iter() {
      team.lock().player_disconnect(id);
    }
    self.players.write().remove(&id);
  }

  /// Returns a read lock on the players map.
  pub fn all_players(&self) -> RwLockReadGuard<'_, HashMap<UUID, (usize, Arc<Player>)>> {
    self.players.read()
  }

  fn global_tick_loop(self: Arc<Self>) {
    let mut start = Instant::now();
    loop {
      // runs on tick() for plugins
      self.events().global_event(event::Tick {});
      // updates after() things
      self.plugins().tick();
      let passed = Instant::now().duration_since(start);
      start += TICK_TIME;
      match TICK_TIME.checked_sub(passed) {
        Some(t) => spin_sleep::sleep(t),
        None => warn!("plugin tick took {passed:?} (more than 50 ms)"),
      }
    }
  }

  pub fn send_to_all(&self, out: impl Into<cb::Packet>) {
    let out = out.into();
    for w in self.worlds.read().iter() {
      for p in w.players().iter() {
        p.send(out.clone());
      }
    }
  }

  pub fn get_player(&self, id: UUID) -> Option<Arc<Player>> {
    self.players.read().get(&id).map(|v| v.1.clone())
  }

  pub fn get_player_username(&self, name: &String) -> Option<Arc<Player>> {
    for (_, (_, p)) in self.players.read().iter() {
      if p.username() == name {
        return Some(Arc::clone(p));
      }
    }
    None
  }

  pub fn save_all(&self) {
    for world in self.worlds.read().iter() {
      world.save();
    }
  }

  #[cfg(not(target_family = "unix"))]
  pub fn stop_on_ctrlc(self: &Arc<Self>) {}
  #[cfg(target_family = "unix")]
  pub fn stop_on_ctrlc(self: &Arc<Self>) {
    use parking_lot::lock_api::RawMutex;

    static CTRLC: Mutex<Option<Arc<WorldManager>>> =
      Mutex::const_new(parking_lot::RawMutex::INIT, None);

    let mut lock = CTRLC.lock();
    if lock.is_some() {
      panic!(
        "cannot stop this worldmanager on ctrlc, as another worldmanager is already registered"
      );
    }
    *lock = Some(self.clone());

    use nix::{
      libc,
      sys::signal::{signal, SigHandler, Signal},
    };

    extern "C" fn handle_sigint(_sig: libc::c_int) {
      let lock = CTRLC.lock();
      println!();
      if let Some(wm) = &*lock {
        wm.save_all();
      }
      std::process::exit(0);
    }

    let handler = SigHandler::Handler(handle_sigint);
    unsafe { signal(Signal::SIGINT, handler) }.unwrap();
  }
}
