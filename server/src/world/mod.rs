mod chunk;

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
  math::{ChunkPos, Pos, PosError, UUID},
  net::{cb, Other},
  proto::Packet,
  version::ProtocolVersion,
};

use crate::{block, net::Connection, player::Player};
use chunk::MultiChunk;

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
  chunks:    RwLock<HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
  players:   Mutex<Vec<Arc<Mutex<Player>>>>,
  eid:       Arc<AtomicI32>,
  converter: Arc<block::Converter>,
}

#[derive(Clone)]
pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds:    Vec<Arc<World>>,
  converter: Arc<block::Converter>,
}

impl World {
  pub fn new(converter: Arc<block::Converter>) -> Self {
    World {
      chunks: RwLock::new(HashMap::new()),
      players: Mutex::new(vec![]),
      eid: Arc::new(1.into()),
      converter,
    }
  }
  async fn new_player(self: Arc<Self>, conn: Arc<Connection>, player: Player) {
    let player = Arc::new(Mutex::new(player));
    self.players.lock().await.push(player.clone());

    let c = conn.clone();
    tokio::spawn(async move {
      // Network recieving task
      c.run().await.unwrap();
    });

    let mut int = time::interval(Duration::from_millis(50));
    tokio::spawn(async move {
      // Player init
      {
        let p = player.lock().await;

        let mut out = cb::Packet::new(cb::ID::Login);
        out.set_i32("eid", self.eid());
        out.set_byte("gamemode", 1); // Creative
        out.set_bool("reduced_debug_info", false); // Don't reduce debug info
        conn.send(out).await;

        for x in -10..10 {
          for z in -10..10 {
            let mut out = cb::Packet::new(cb::ID::MapChunk);
            self.chunk(ChunkPos::new(x, z), |c| {
              let mut pb = c.to_proto(p.ver().block());
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
        let p = player.lock().await;
        // Do player collision and packets and stuff
        info!("player tick for {}", p.username());
        if p.conn().closed() {
          break;
        }
        // Once per second, send keep alive packet
        if tick % 20 == 0 {
          let mut out = cb::Packet::new(cb::ID::KeepAlive);
          out.set_i32("keep_alive_id", 1234556);
          conn.send(out).await;
        }
        tick += 1;
      }
    });
  }

  /// Returns a new, unique EID.
  pub fn eid(&self) -> i32 {
    self.eid.fetch_add(1, Ordering::SeqCst)
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
      chunks
        .entry(pos)
        .or_insert_with(|| Arc::new(StdMutex::new(MultiChunk::new(self.converter.clone()))));
    }
    let chunks = self.chunks.read().unwrap();
    let c = chunks[&pos].lock().unwrap();
    f(c)
  }

  /// This sets a block within the world. It will return an error if the
  /// position is outside of the world.
  pub fn set_block(&self, pos: Pos, ty: block::Type) -> Result<(), PosError> {
    self.chunk(pos.chunk(), |mut c| c.set_block(pos.chunk_rel(), &ty))
  }
}

impl Default for WorldManager {
  fn default() -> Self {
    WorldManager::new()
  }
}

impl WorldManager {
  pub fn new() -> Self {
    let mut w = WorldManager { converter: Arc::new(block::Converter::new()), worlds: vec![] };
    w.add_world();
    w
  }

  pub fn add_world(&mut self) {
    self.worlds.push(Arc::new(World::new(self.converter.clone())));
  }

  /// Returns the current converter. This can be used to convert old block ids
  /// to new ones, and vice versa.
  pub fn get_converter(&self) -> &block::Converter {
    &self.converter
  }

  /// Adds a new player into the game. This should be called when a new grpc
  /// proxy connects.
  pub async fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    // Default world. Might want to change this later, but for now this is easiest.
    // TODO: Player name, uuid
    let conn = Arc::new(Connection::new(req, tx));
    let w = self.worlds[0].clone();
    let player = Player::new(
      w.eid(),
      "macmv".into(),
      UUID::from_u128(0x1111111),
      conn.clone(),
      ProtocolVersion::V1_8,
    );
    w.new_player(conn, player).await;
  }
}
