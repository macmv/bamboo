use crate::{
  net::{packet, ConnSender},
  player::Player,
  world::WorldManager,
};
use bb_common::{
  math::FPos,
  net::sb,
  util::{JoinInfo, JoinMode, UUID},
  version::ProtocolVersion,
};
use std::sync::Arc;

pub struct TestHandler {
  wm:     Arc<WorldManager>,
  player: Arc<Player>,
}

impl TestHandler {
  pub fn new() -> Self {
    let wm = Arc::new(WorldManager::new());
    let sender = ConnSender::new();
    let info = JoinInfo {
      mode:     JoinMode::New,
      username: "macmv".into(),
      uuid:     UUID::from_u128(0),
      ver:      ProtocolVersion::V1_8.id(),
    };
    let player = wm.new_player(sender, info);
    TestHandler { wm, player }
  }
  pub fn handle(&self, p: sb::Packet) { packet::handle(&self.wm, &self.player, p); }
  pub fn player(&self) -> &Arc<Player> { &self.player }
}

#[test]
fn test_move_packets() {
  let sender = TestHandler::new();
  // sender.handle(sb::Packet::PlayerMove {});
  let pos = sender.player().lock_pos();
  assert_eq!(pos.next, FPos::new(1.0, 2.0, 3.0));
}
