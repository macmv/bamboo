pub mod clientbound;
pub mod serverbound;

use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::Sender;
use tonic::{Status, Streaming};

use common::proto;

pub struct Connection {
  rx:     Streaming<proto::Packet>,
  tx:     Sender<Result<proto::Packet, Status>>,
  closed: AtomicBool,
}

impl Connection {
  pub fn new(rx: Streaming<proto::Packet>, tx: Sender<Result<proto::Packet, Status>>) -> Self {
    Connection { rx, tx, closed: false.into() }
  }

  pub async fn run(&mut self) -> Result<(), Status> {
    'running: loop {
      let p = match self.rx.message().await? {
        Some(p) => serverbound::Packet::from(p),
        None => break 'running,
      };
      info!("got packet from client {:?}", p);
    }
    self.closed.store(true, Ordering::SeqCst);
    Ok(())
  }

  pub fn closed(&self) -> bool {
    self.closed.load(Ordering::SeqCst)
  }
}
