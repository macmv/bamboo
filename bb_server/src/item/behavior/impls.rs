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
  fn break_block(&self, mut block: Block, _: &Arc<Player>) -> bool {
    let mut all_props: Vec<_> = block.ty.props().into_iter().collect();
    all_props.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    for (key, _) in all_props {
      let prop = block.ty.prop_at(&key).unwrap();
      let val = prop.id_of(&block.ty.prop(&key));
      if val == prop.len() - 1 {
        let new_val = prop.from_id(0);
        block.ty.set_prop(&key, new_val);
      } else {
        let new_val = prop.from_id(val + 1);
        block.ty.set_prop(&key, new_val);
        let _ = block.world.set_block(block.pos, block.ty);
        return true;
      }
    }
    let _ = block.world.set_block(block.pos, block.ty);
    true
  }
}
