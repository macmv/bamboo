mod chunk;

use std::{
  collections::HashMap,
  future::Future,
  marker::PhantomData,
  ops::Deref,
  sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex as StdMutex, MutexGuard as StdMutexGuard,
  },
  time::Duration,
};
use tokio::{
  sync::{mpsc::Sender, Mutex, MutexGuard, RwLock, RwLockReadGuard},
  time,
};
use tonic::{Status, Streaming};

use common::{
  math::{ChunkPos, UUID},
  net::cb,
  proto::Packet,
  version::ProtocolVersion,
};

use crate::{net::Connection, player::Player};
use chunk::MultiChunk;

pub struct ChunkRef<'a> {
  pos:    ChunkPos,
  // Need to keep this is scope while we mess with the chunk
  chunks: RwLockReadGuard<'a, HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
}

impl ChunkRef<'_> {
  fn lock<'a>(&'a self) -> &'a MultiChunk {
    &self.chunks.get(&self.pos).unwrap().lock().unwrap()
  }
}

pub struct World {
  chunks:  RwLock<HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
  players: Mutex<Vec<Arc<Mutex<Player>>>>,
  eid:     Arc<AtomicU32>,
}

#[derive(Clone)]
pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds: Vec<Arc<World>>,
}

impl World {
  pub fn new() -> Self {
    World {
      chunks:  RwLock::new(HashMap::new()),
      players: Mutex::new(vec![]),
      eid:     Arc::new(1.into()),
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
          out.set_i32(0, 1234556);
          conn.send(out).await;
        }
        for x in -10..10 {
          for z in -10..10 {
            let mut out = cb::Packet::new(cb::ID::ChunkData);
            {
              let chunk = self.chunk(ChunkPos::new(x, z)).await.lock();
              out.set_other(&chunk.to_proto(p.ver().block())).unwrap();
            }
            conn.send(out).await;
          }
        }
        tick += 1;
      }
    });
  }

  /// Returns a new, unique EID.
  pub fn eid(&self) -> u32 {
    self.eid.fetch_add(1, Ordering::SeqCst)
  }

  /// Returns a locked Chunk. This will generate a new chunk if there is not one
  /// stored there.
  pub async fn chunk<'a>(&'a self, pos: ChunkPos) -> ChunkRef<'a> {
    // We first check (read-only) if we need to generate a new chunk
    if !self.chunks.read().await.contains_key(&pos) {
      // If we do, we lock it for writing
      let mut chunks = self.chunks.write().await;
      // Make sure that we didn't get a race condition
      if !chunks.contains_key(&pos) {
        // And finally generate the chunk.
        chunks.insert(pos, Arc::new(StdMutex::new(MultiChunk::new())));
      }
    }
    let chunks = self.chunks.read().await;
    ChunkRef { chunks, pos }
  }
}

impl WorldManager {
  pub fn new() -> Self {
    WorldManager { worlds: vec![Arc::new(World::new())] }
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
