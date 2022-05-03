use super::Behavior;
use crate::{block::Block, player::Player};
use bb_common::util::Chat;
use std::sync::Arc;

pub struct DebugStick;
impl Behavior for DebugStick {
  fn interact_block(&self, block: Block, player: &Arc<Player>) -> bool {
    player.send_hotbar(Chat::new(block.ty.to_string()));
    true
  }
}
