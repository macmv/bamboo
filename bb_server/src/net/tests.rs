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
    let wm = Arc::new(WorldManager::new(false));
    let world = wm.new_world();
    let world = wm.add_world_no_tick(world);
    world.init();
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
  // This is a useful but unused function.
  #[allow(unused)]
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
  let pos = handler.player().pos();
  let new_pos = pos + FPos::new(0.0, 1.0, 0.0);
  handler.handle(sb::Packet::PlayerPos {
    x:         new_pos.x(),
    y:         new_pos.y(),
    z:         new_pos.z(),
    on_ground: true,
  });
  {
    let pos = handler.player().lock_pos();
    assert_eq!(pos.next, new_pos);
    handler.assert_empty();
  }
  handler.player().tick();
  {
    let pos = handler.player().lock_pos();
    assert_eq!(pos.curr, new_pos);
    assert_eq!(pos.next, new_pos);
    handler.assert_empty();
  }
}
