use tokio::sync::mpsc::Sender;
use tonic::{Status, Streaming};

use common::proto::Packet;

pub struct Connection {
  rx: Streaming<Packet>,
  tx: Sender<Result<Packet, Status>>,
}

impl Connection {
  pub fn new(rx: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) -> Self {
    Connection { rx, tx }
  }

  pub async fn run(&mut self) -> Result<(), Status> {
    'running: loop {
      let p = Packet::from(self.rx.message().await?);
    }
  }
}
