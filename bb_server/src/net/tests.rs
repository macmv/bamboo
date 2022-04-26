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
  /// Creates a new testing handler, without any init packets in the buffer.
  pub fn new() -> Self {
    let sender = Self::new_with_init();
    sender.clear();
    sender
  }
  /// Creates a new testing handler, with the init packets in the buffer.
  pub fn new_with_init() -> Self {
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
        info!("packet: {m:?}");
      }
      panic!("got packets, but expected none");
    }
  }
  pub fn assert_sent(&self, expected_packets: &[cb::Packet]) {
    let mut actual_packets = vec![];
    while let Ok(p) = self.rx.try_recv() {
      actual_packets.push(p);
    }
    let mut equal = actual_packets.len() == expected_packets.len();
    for p in expected_packets {
      if !actual_packets.contains(p) {
        equal = false;
        break;
      }
    }
    if !equal {
      error!("actual and expected packets were not equal:");
      if actual_packets.is_empty() {
        info!("no actual packets");
      }
      for p in actual_packets {
        info!("actual packet: {p:?}");
      }
      if expected_packets.is_empty() {
        info!("no expected packets");
      }
      for p in expected_packets {
        info!("expected packet: {p:?}");
      }
      panic!("actual and expected packets were not equal");
    }
  }
}

#[test]
fn test_move_packets() {
  let handler = TestHandler::new();
  handler.handle(sb::Packet::PlayerPos {
    x:         1.0,
    y:         2.0,
    z:         3.0,
    on_ground: true,
  });
  {
    let pos = handler.player().lock_pos();
    assert_eq!(pos.next, FPos::new(1.0, 2.0, 3.0));
    handler.assert_empty();
  }
  handler.player().tick();
  handler.assert_sent(&[cb::Packet::EntityMove {
    eid:       handler.player().eid(),
    x:         1 * 32,
    y:         2 * 32,
    z:         3 * 32,
    on_ground: true,
  }]);
}
