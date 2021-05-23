mod chunk;
mod gen;

use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicI32, Ordering},
    Arc, Mutex as StdMutex, MutexGuard as StdMutexGuard, RwLock,
  },
  time::Duration,
};
use tokio::{
  sync::{mpsc::Sender, Mutex},
  time,
};
use tonic::{Status, Streaming};

use common::{
  math::{ChunkPos, FPos, Pos, PosError, UUID},
  net::{cb, Other},
  proto::Packet,
  util::Chat,
  version::ProtocolVersion,
};

use crate::{block, item, net::Connection, player::Player};
use chunk::MultiChunk;
use gen::Generator;

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
  chunks:          RwLock<HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
  players:         Mutex<HashMap<UUID, Arc<Player>>>,
  eid:             Arc<AtomicI32>,
  block_converter: Arc<block::TypeConverter>,
  item_converter:  Arc<item::TypeConverter>,
  generator:       StdMutex<Generator>,
}

pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds:          Vec<Arc<World>>,
  block_converter: Arc<block::TypeConverter>,
  item_converter:  Arc<item::TypeConverter>,
}

impl World {
  pub fn new(
    block_converter: Arc<block::TypeConverter>,
    item_converter: Arc<item::TypeConverter>,
  ) -> Self {
    World {
      chunks: RwLock::new(HashMap::new()),
      players: Mutex::new(HashMap::new()),
      eid: Arc::new(1.into()),
      block_converter,
      item_converter,
      generator: StdMutex::new(Generator::new()),
    }
  }
  async fn new_player(self: Arc<Self>, player: Player) {
    let conn = player.clone_conn();
    let player = Arc::new(player);
    self.players.lock().await.insert(player.id(), player.clone());

    // Network recieving task
    let c = conn.clone();
    let p = player.clone();
    tokio::spawn(async move {
      c.run(&p).await.unwrap();
    });

    // Player tick loop
    let mut int = time::interval(Duration::from_millis(50));
    tokio::spawn(async move {
      info!("{} has logged in", player.username());
      // Player init
      {
        let mut out = cb::Packet::new(cb::ID::Login);
        out.set_i32("entity_id", self.eid());
        out.set_byte("game_mode", 1); // Creative
        out.set_byte("difficulty", 1); // Normal
        out.set_byte("dimension", 0); // Overworld
        out.set_str("level_type", "default".into());
        out.set_byte("max_players", 0); // Ignored
        out.set_bool("reduced_debug_info", false); // Don't reduce debug info
        conn.send(out).await;

        for x in -10..10 {
          for z in -10..10 {
            let mut out = cb::Packet::new(cb::ID::MapChunk);
            self.chunk(ChunkPos::new(x, z), |c| {
              let mut pb = c.to_proto(player.ver().block());
              pb.x = x;
              pb.z = z;
              out.set_other(Other::Chunk(pb)).unwrap();
            });
            conn.send(out).await;
          }
        }

        let mut out = cb::Packet::new(cb::ID::Position);
        out.set_f64("x", 0.0); // X
        out.set_f64("y", 60.0); // Y
        out.set_f64("z", 0.0); // Z
        out.set_f32("yaw", 0.0); // Yaw
        out.set_f32("pitch", 0.0); // Pitch
        out.set_byte("flags", 0); // Flags
        out.set_i32("teleport_id", 1234); // TP id
        conn.send(out).await;
      }
      // Player tick loop
      let mut tick = 0;
      loop {
        int.tick().await;
        if conn.closed() {
          // TODO: Close any other tasks for this player
          break;
        }
        // Updates the player correctly, and performs collision checks. This also
        // handles new chunks.
        player.tick();
        // Do player collision and packets and stuff
        // Once per second, send keep alive packet
        if tick % 20 == 0 {
          let mut out = cb::Packet::new(cb::ID::KeepAlive);
          out.set_i32("keep_alive_id", 1234556);
          conn.send(out).await;
        }
        tick += 1;
      }
      info!("{} has logged out", player.username());
      self.players.lock().await.remove(&player.id());
    });
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
        self.generator.lock().unwrap().generate(&mut c);
        Arc::new(StdMutex::new(c))
      });
    }
    let chunks = self.chunks.read().unwrap();
    let c = chunks[&pos].lock().unwrap();
    f(c)
  }

  /// This sets a block within the world. It will return an error if the
  /// position is outside of the world.
  pub async fn set_block(&self, pos: Pos, ty: &block::Type) -> Result<(), PosError> {
    self.chunk(pos.chunk(), |mut c| c.set_type(pos.chunk_rel(), ty))?;

    for p in self.players.lock().await.values() {
      let mut out = cb::Packet::new(cb::ID::BlockChange);
      out.set_pos("location", pos);
      out.set_i32("type", self.block_converter.to_old(ty.id(), p.ver().block()) as i32);
      p.conn().send(out).await;
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
  pub async fn broadcast(&self, msg: &Chat) {
    let mut out = cb::Packet::new(cb::ID::Chat);
    out.set_str("message", msg.to_json());
    out.set_byte("position", 0); // Chat box, not over hotbar

    for p in self.players.lock().await.values() {
      p.conn().send(out.clone()).await;
    }
  }
}

impl Default for WorldManager {
  fn default() -> Self {
    WorldManager::new()
  }
}

impl WorldManager {
  pub fn new() -> Self {
    let mut w = WorldManager {
      block_converter: Arc::new(block::TypeConverter::new()),
      item_converter:  Arc::new(item::TypeConverter::new()),
      worlds:          vec![],
    };
    w.add_world();
    w
  }

  pub fn add_world(&mut self) {
    self
      .worlds
      .push(Arc::new(World::new(self.block_converter.clone(), self.item_converter.clone())));
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

  /// Adds a new player into the game. This should be called when a new grpc
  /// proxy connects.
  pub async fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    let conn = Arc::new(Connection::new(req, tx));
    let (username, uuid) = conn.wait_for_login().await;
    let w = self.worlds[0].clone();
    let player = Player::new(
      w.eid(),
      username,
      uuid,
      conn,
      ProtocolVersion::V1_8,
      w.clone(),
      FPos::new(0.0, 60.0, 0.0),
    );
    w.new_player(player).await;
  }
}
