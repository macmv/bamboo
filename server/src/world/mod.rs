mod chunk;

use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
  time::Duration,
};
use tokio::{sync::mpsc::Sender, time};
use tonic::{Status, Streaming};

use common::{math::UUID, proto::Packet};

use crate::{net::Connection, player::Player};

pub struct World {
  chunks: HashMap<chunk::Pos, Mutex<chunk::Chunk>>,
}

#[derive(Clone)]
pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds: Vec<Arc<World>>,
  eid:    u32,
}

impl World {
  pub fn new() -> Self {
    World { chunks: HashMap::new() }
  }
  fn new_player(&self, player: Player) {}
}

impl WorldManager {
  pub fn new() -> Self {
    WorldManager { worlds: vec![Arc::new(World::new())], eid: 1 }
  }

  pub fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    let world = self.worlds[0].clone();
    tokio::spawn(async move {
      // TODO: EID counter
      let mut player =
        Player::new(1, "macmv".into(), UUID::from_u128(0xff1232452345), Connection::new(req, tx));
      // self.eid += 1;

      // Default world. Might want to change this later, but for now this is easiest.
      world.new_player(player);
      // match conn.run().await {
      //   Ok(_) => {}
      //   Err(e) => {
      //     error!("error in connection: {}", e);
      //   }
      // }
      //
      // loop {
      //   let mut p = Packet::default();
      //   p.id = 10;
      //   println!("  => send {:?}", p);
      //   tx.send(Ok(p)).await.unwrap();
      //
      //   time::sleep(Duration::from_secs(1)).await;
      // }
    });
  }
}
