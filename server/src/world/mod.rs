pub mod chunk;
pub mod gen;
mod init;
mod players;

use std::{
  collections::HashMap,
  convert::TryInto,
  sync::{
    atomic::{AtomicI32, AtomicU32, Ordering},
    Arc, Mutex as StdMutex, MutexGuard as StdMutexGuard, RwLock,
  },
  time::{Duration, Instant},
};
use tokio::{
  sync::{mpsc::Sender, Mutex, MutexGuard},
  time,
};
use tonic::{Status, Streaming};

use common::{
  math::{ChunkPos, FPos, Pos, PosError},
  net::cb,
  proto::Packet,
  util::{
    chat::{Chat, Color},
    UUID,
  },
  version::BlockVersion,
};

use crate::{block, command::CommandTree, entity, item, net::Connection, player::Player, plugin};
use chunk::MultiChunk;
use gen::WorldGen;

pub use players::{PlayersIter, PlayersMap};

// pub struct ChunkRef<'a> {
//   pos:    ChunkPos,
//   // Need to keep this is scope while we mess with the chunk
//   chunks: RwLockReadGuard<'a, HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
// }
//
// impl ChunkRef<'_> {
//   fn lock<'a>(&'a self) -> StdMutexGuard<'a, MultiChunk> {
//     self.chunks.get(&self.pos).unwrap().lock().unwrap()
//   }
// }

pub struct World {
  chunks:           RwLock<HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
  players:          Mutex<PlayersMap>,
  eid:              AtomicI32,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
  commands:         CommandTree,
  generator:        StdMutex<WorldGen>,
  mspt:             AtomicU32,
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
}

impl World {
  pub fn new(
    block_converter: Arc<block::TypeConverter>,
    item_converter: Arc<item::TypeConverter>,
    entity_converter: Arc<entity::TypeConverter>,
    plugins: Arc<plugin::PluginManager>,
    wm: Arc<WorldManager>,
  ) -> Arc<Self> {
    let world = Arc::new(World {
      chunks: RwLock::new(HashMap::new()),
      players: Mutex::new(PlayersMap::new()),
      eid: 1.into(),
      block_converter,
      item_converter,
      entity_converter,
      plugins,
      commands: CommandTree::new(),
      generator: StdMutex::new(WorldGen::new()),
      mspt: 0.into(),
      wm,
    });
    let w = world.clone();
    tokio::spawn(async move {
      w.init().await;
      w.global_tick_loop().await;
    });
    world
  }
  async fn global_tick_loop(self: Arc<Self>) {
    let mut int = time::interval(Duration::from_millis(50));
    let mut tick = 0;
    loop {
      int.tick().await;
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
        for p in self.players.lock().await.values() {
          p.conn().send(out.clone()).await;
        }
      }
      tick += 1;
    }
  }
  async fn new_player(self: Arc<Self>, player: Player) {
    let conn = player.clone_conn();
    let player = Arc::new(player);
    {
      let mut p = self.players.lock().await;
      if p.contains_key(&player.id()) {
        player.disconnect("Another player with the same id is already connected!").await;
        return;
      }
      p.insert(player.id(), player.clone());
    }

    // Network recieving task
    let c = conn.clone();
    let p = player.clone();
    let wm = self.wm.clone();
    tokio::spawn(async move {
      c.run(p, wm).await.unwrap();
    });

    // Player tick loop
    tokio::spawn(async move {
      let name = player.username().to_string();
      let id = player.id();
      info!("{} has logged in", name);
      self.player_loop(player, conn).await;
      info!("{} has logged out", name);
      self.players.lock().await.remove(&id);
    });
  }

  async fn player_loop(&self, player: Arc<Player>, conn: Arc<Connection>) {
    let mut int = time::interval(Duration::from_millis(50));
    // Player init
    self.player_init(&player, &conn).await;
    // Player tick loop
    let mut tick = 0;
    loop {
      int.tick().await;
      if conn.closed() {
        // TODO: Close any other tasks for this player
        break;
      }
      let start = Instant::now();
      // Updates the player correctly, and performs collision checks. This also
      // handles new chunks.
      player.tick().await;
      // Do player collision and packets and stuff
      // Once per second, send keep alive packet
      if tick % 20 == 0 {
        conn
          .send(cb::Packet::KeepAlive {
            keep_alive_id_v1_8:    Some(1234556),
            keep_alive_id_v1_12_2: Some(1234556),
          })
          .await;
      }
      tick += 1;
      self.mspt.fetch_add(start.elapsed().as_millis().try_into().unwrap(), Ordering::SeqCst);
    }
  }

  /// Returns a new, unique EID.
  pub fn eid(&self) -> i32 {
    self.eid.fetch_add(1, Ordering::SeqCst)
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn get_block_converter(&self) -> &block::TypeConverter {
    &self.block_converter
  }
  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn get_item_converter(&self) -> &item::TypeConverter {
    &self.item_converter
  }
  /// Returns the current entity converter. This can be used to convert old
  /// entity ids to new ones, and vice versa.
  pub fn get_entity_converter(&self) -> &entity::TypeConverter {
    &self.entity_converter
  }
  /// Returns the plugin manager. This is how events can be sent to plugins.
  pub fn get_plugins(&self) -> &plugin::PluginManager {
    &self.plugins
  }
  /// Returns the command tree that the server uses. This can be used to add
  /// custom commands to the server.
  pub fn get_commands(&self) -> &CommandTree {
    &self.commands
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
    F: FnOnce(StdMutexGuard<MultiChunk>) -> R,
  {
    // We first check (read-only) if we need to generate a new chunk
    if !self.chunks.read().unwrap().contains_key(&pos) {
      // If we do, we lock it for writing
      let mut chunks = self.chunks.write().unwrap();
      // Make sure that the chunk was not written in between locking this chunk
      chunks.entry(pos).or_insert_with(|| {
        let mut c = MultiChunk::new(self.block_converter.clone());
        self.generator.lock().unwrap().generate(pos, &mut c);
        Arc::new(StdMutex::new(c))
      });
    }
    let chunks = self.chunks.read().unwrap();
    let c = chunks[&pos].lock().unwrap();
    f(c)
  }

  /// This serializes a chunk for the given version. This packet can be sent
  /// directly to a client. Note that on most vanilla versions, sending a chunk
  /// to a client that already has loaded that chunk will cause a memory leak.
  /// Unloading a chunk multiple times will not cause a memory leak. If you are
  /// trying to re-send an entire chunk to a player, make sure to send them an
  /// unload chunk packet first. Use at your own risk!
  pub fn serialize_chunk(&self, pos: ChunkPos, ver: BlockVersion) -> cb::Packet {
    self.chunk(pos, |c| crate::net::serialize::serialize_chunk(pos, &c, ver))
  }

  /// This sets a block within the world. It will return an error if the
  /// position is outside of the world. Unlike
  /// [`MultiChunk::set_type`](chunk::MultiChunk::set_type), this will send
  /// packets to anyone within render distance of the given chunk.
  pub async fn set_block(&self, pos: Pos, ty: &block::Type) -> Result<(), PosError> {
    self.chunk(pos.chunk(), |mut c| c.set_type(pos.chunk_rel(), ty))?;

    for p in self.players().await.iter().in_view(pos.chunk()) {
      p.conn()
        .send(cb::Packet::BlockChange {
          location: pos,
          type_:    self.block_converter.to_old(ty.id(), p.ver().block()) as i32,
        })
        .await;
    }
    Ok(())
  }

  /// This sets a block within the world. This will use the default type of the
  /// given kind. It will return an error if the position is outside of the
  /// world.
  pub async fn set_kind(&self, pos: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.set_block(pos, self.block_converter.get(kind).default_type()).await
  }

  /// This broadcasts a chat message to everybody in the world.
  pub async fn broadcast<M: Into<Chat>>(&self, msg: M) {
    let out = cb::Packet::Chat {
      message:      msg.into().to_json(),
      position:     0, // Chat box, not above hotbar
      sender_v1_16: Some(UUID::from_u128(0)),
    };

    for p in self.players.lock().await.values() {
      p.conn().send(out.clone()).await;
    }
  }

  // Runs f for all players within render distance of the chunk.
  pub async fn players(&self) -> MutexGuard<'_, PlayersMap> {
    self.players.lock().await
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
      worlds:           Mutex::new(vec![]),
    }
  }

  pub async fn run(self: Arc<Self>) {
    self.plugins.clone().run(self).await;
  }

  /// Adds a new world. Currently, this requires a mutable reference, which
  /// cannot be obtained outside of initialization.
  pub async fn add_world(self: &Arc<Self>) {
    self.worlds.lock().await.push(World::new(
      self.block_converter.clone(),
      self.item_converter.clone(),
      self.entity_converter.clone(),
      self.plugins.clone(),
      self.clone(),
    ));
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn get_block_converter(&self) -> &block::TypeConverter {
    &self.block_converter
  }

  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn get_item_converter(&self) -> &item::TypeConverter {
    &self.item_converter
  }

  /// Broadcasts a message to everyone one the server.
  pub async fn broadcast<M: Into<Chat>>(&self, msg: M) {
    let out = cb::Packet::Chat {
      message:      msg.into().to_json(),
      position:     0, // Chat box, not above hotbar
      sender_v1_16: Some(UUID::from_u128(0)),
    };

    let worlds = self.worlds.lock().await;
    for w in worlds.iter() {
      for p in w.players.lock().await.values() {
        p.conn().send(out.clone()).await;
      }
    }
  }

  /// Adds a new player into the game. This should be called when a new grpc
  /// proxy connects.
  pub async fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    let mut conn = Connection::new(req, tx);
    let (username, uuid, ver) = conn.wait_for_login().await;
    let w = self.worlds.lock().await[0].clone();
    let player = Player::new(
      w.eid(),
      username,
      uuid,
      Arc::new(conn),
      ver,
      w.clone(),
      FPos::new(0.0, 60.0, 0.0),
    );
    w.new_player(player).await;
  }
}
