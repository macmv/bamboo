mod chunk;

use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::{
  sync::{mpsc::Sender, Mutex},
  time,
};
use tonic::{Status, Streaming};

use common::{
  math::{ChunkPos, UUID},
  net::cb,
  proto::Packet,
};

use crate::{net::Connection, player::Player};

pub struct World {
  chunks:  HashMap<ChunkPos, Mutex<chunk::MultiChunk>>,
  players: Vec<Arc<Mutex<Player>>>,
  eid:     Arc<AtomicU32>,
}

#[derive(Clone)]
pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds: Vec<Arc<Mutex<World>>>,
}

impl World {
  pub fn new() -> Self {
    World { chunks: HashMap::new(), players: vec![], eid: Arc::new(1.into()) }
  }
  fn new_player(&mut self, username: String, id: UUID, conn: Connection) {
    let conn = Arc::new(conn);
    let player = Arc::new(Mutex::new(Player::new(self.eid(), username, id, conn.clone())));
    self.players.push(player.clone());

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
        let player = player.lock().await;
        // Do player collision and packets and stuff
        info!("player tick for {}", player.username());
        if player.conn().closed() {
          break;
        }
        // Once per second, send keep alive packet
        if tick % 20 == 0 {
          let mut out = cb::Packet::new(cb::ID::KeepAlive);
          out.set_i32(0, 1234556);
          conn.send(out).await;
        }
        tick += 1;
      }
    });
  }

  // Returns a new, unique EID.
  pub fn eid(&self) -> u32 {
    self.eid.fetch_add(1, Ordering::SeqCst)
  }
}

impl WorldManager {
  pub fn new() -> Self {
    WorldManager { worlds: vec![Arc::new(Mutex::new(World::new()))] }
  }

  /// Adds a new player into the game. This should be called when a new grpc
  /// proxy connects.
  pub async fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    // Default world. Might want to change this later, but for now this is easiest.
    // TODO: Player name, uuid
    self.worlds[0].lock().await.new_player(
      "macmv".into(),
      UUID::from_u128(0x1111111),
      Connection::new(req, tx),
    );
  }
}
