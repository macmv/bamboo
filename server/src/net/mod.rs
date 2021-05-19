use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc::Sender, Mutex};
use tonic::{Status, Streaming};

use common::{
  math::UUID,
  net::{cb, sb},
  proto,
};

use crate::{block, player::Player};

pub struct Connection {
  rx:     Mutex<Streaming<proto::Packet>>,
  tx:     Mutex<Sender<Result<proto::Packet, Status>>>,
  closed: AtomicBool,
}

impl Connection {
  pub(crate) fn new(
    rx: Streaming<proto::Packet>,
    tx: Sender<Result<proto::Packet, Status>>,
  ) -> Self {
    Connection { rx: Mutex::new(rx), tx: Mutex::new(tx), closed: false.into() }
  }

  /// This waits for the a login packet from the proxy. If any other packet is
  /// recieved, this will panic. This should only be called right after a
  /// connection is created.
  pub(crate) async fn wait_for_login(&self) -> (String, UUID) {
    let p = match self.rx.lock().await.message().await.unwrap() {
      Some(p) => sb::Packet::from_proto(p),
      None => panic!("connection was closed while listening for a login packet"),
    };
    match p.id() {
      sb::ID::Login => (p.get_str("username").into(), p.get_uuid("uuid")),
      _ => panic!("expecting login packet, got: {}", p),
    }
  }

  /// This starts up the recieving loop for this connection. Do not call this
  /// more than once.
  pub(crate) async fn run(&self, player: &Mutex<Player>) -> Result<(), Status> {
    'running: loop {
      let p = match self.rx.lock().await.message().await? {
        Some(p) => sb::Packet::from_proto(p),
        None => break 'running,
      };
      match p.id() {
        sb::ID::BlockDig => {
          let pos = p.get_pos("location");
          info!("digging at {}", &pos);
          player.lock().await.world().set_kind(pos, block::Kind::Air).unwrap();
        }
        sb::ID::Position => {
          let mut player = player.lock().await;
          player.set_next_pos(p.get_double("x"), p.get_double("y"), p.get_double("z"));
        }
        sb::ID::PositionLook => {
          let mut player = player.lock().await;
          player.set_next_pos(p.get_double("x"), p.get_double("y"), p.get_double("z"));
          player.set_next_look(p.get_float("yaw"), p.get_float("pitch"));
        }
        sb::ID::Look => {
          let mut player = player.lock().await;
          player.set_next_look(p.get_float("yaw"), p.get_float("pitch"));
        }
        // _ => warn!("got unknown packet from client: {:?}", p),
        _ => (),
      }
      // info!("got packet from client {:?}", p);
    }
    self.closed.store(true, Ordering::SeqCst);
    Ok(())
  }

  /// Sends a packet to the proxy, which will then get sent to the client.
  pub async fn send(&self, p: cb::Packet) {
    self.tx.lock().await.send(Ok(p.into_proto())).await.unwrap();
  }

  // Returns true if the connection has been closed.
  pub fn closed(&self) -> bool {
    self.closed.load(Ordering::SeqCst)
  }
}
