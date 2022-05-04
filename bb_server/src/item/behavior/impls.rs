use super::Behavior;
use crate::{block, block::Block, player::Click};
use bb_common::util::Chat;

pub struct DebugStick;
impl Behavior for DebugStick {
  fn interact_block(&self, block: Block, click: Click) -> bool {
    click.player.send_hotbar(Chat::new(block.ty.to_string()));
    true
  }
  #[allow(clippy::collapsible_else_if)]
  fn break_block(&self, mut block: Block, click: Click) -> bool {
    let mut all_props: Vec<_> = block.ty.props().into_iter().collect();
    all_props.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let reverse = click.player.is_crouching();
    for (key, _) in all_props {
      let prop = block.ty.prop_at(&key).unwrap();
      let val = prop.id_of(&block.ty.prop(&key));
      if reverse {
        if val == 0 {
          let new_val = prop.from_id(prop.len() - 1);
          block.ty.set_prop(&key, new_val);
        } else {
          let new_val = prop.from_id(val - 1);
          block.ty.set_prop(&key, new_val);
          let _ = block.world.set_block(block.pos, block.ty);
          return true;
        }
      } else {
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
    }
    let _ = block.world.set_block(block.pos, block.ty);
    true
  }
}

pub struct Bucket(pub Option<block::Kind>);
impl Behavior for Bucket {
  fn interact_block(&self, _: Block, click: Click) -> bool {
    if self.0.is_none() {
      let result = click.do_raycast(5.0, true);
      dbg!(result);
    }
    true
  }
}
