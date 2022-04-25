use super::WakeEvent;
use crate::{
  net::{packet, ConnSender},
  player::Player,
  world::WorldManager,
};
use bb_common::{
  math::FPos,
  net::{cb, sb},
  util::{JoinInfo, JoinMode, UUID},
  version::ProtocolVersion,
};
use crossbeam_channel::Receiver;
use std::sync::Arc;

pub struct TestHandler {
  rx:      Receiver<cb::Packet>,
  wake_rx: Receiver<WakeEvent>,
  wm:      Arc<WorldManager>,
  player:  Arc<Player>,
}

impl TestHandler {
  pub fn new() -> Self {
    bb_common::init("test");
    let wm = Arc::new(WorldManager::new());
    wm.add_world();
    let poll = mio::Poll::new().unwrap();
    let (rx, wake_rx, sender) = ConnSender::mock(&poll);
    let info = JoinInfo {
      mode:     JoinMode::New,
      username: "macmv".into(),
      uuid:     UUID::from_u128(0),
      ver:      ProtocolVersion::V1_8.id(),
    };
    let player = wm.new_player(sender, info);
    TestHandler { rx, wake_rx, wm, player }
  }
  pub fn handle(&self, p: sb::Packet) { packet::handle(&self.wm, &self.player, p); }
  pub fn player(&self) -> &Arc<Player> { &self.player }
  pub fn clear(&self) {
    while let Ok(_) = self.rx.try_recv() {}
    while let Ok(_) = self.wake_rx.try_recv() {}
  }
  pub fn assert_empty(&self) {
    if !self.rx.is_empty() {
      while let Ok(m) = self.rx.try_recv() {
        info!("message: {m:?}");
      }
      panic!("got messages, but expected none");
    }
  }
}

#[test]
fn test_move_packets() {
  let sender = TestHandler::new();
  sender.clear();
  sender.handle(sb::Packet::PlayerPos {
    x:         1.0,
    y:         2.0,
    z:         3.0,
    on_ground: true,
  });
  let pos = sender.player().lock_pos();
  assert_eq!(pos.next, FPos::new(1.0, 2.0, 3.0));
  sender.assert_empty();
}
