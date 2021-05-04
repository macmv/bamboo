mod chunk;

use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
  time::Duration,
};
use tokio::{sync::mpsc::Sender, time};
use tonic::{Status, Streaming};

use common::proto::Packet;

use crate::net::Connection;

pub struct World {
  chunks: HashMap<chunk::Pos, Mutex<chunk::Chunk>>,
}

#[derive(Clone)]
pub struct WorldManager {
  worlds: Vec<Arc<World>>,
}

impl World {
  pub fn new() -> Self {
    World { chunks: HashMap::new() }
  }
}

impl WorldManager {
  pub fn new() -> Self {
    WorldManager { worlds: Vec::new() }
  }

  pub fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    tokio::spawn(async move {
      let mut conn = Connection::new(req, tx);
      match conn.run().await {
        Ok(_) => {}
        Err(e) => {
          error!("error in connection: {}", e);
        }
      }
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
