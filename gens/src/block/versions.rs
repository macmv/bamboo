use super::{Block, BlockVersion};
use std::collections::HashMap;

pub(super) fn generate(latest: &BlockVersion, old: &BlockVersion) -> (Vec<u32>, Vec<(u32, u32)>) {
  let mut to_old = vec![];
  let mut to_new = vec![]; // This is a hashmap, but is stored as a list of tuples

  let old_blocks: HashMap<String, Block> =
    old.blocks.iter().cloned().map(|b| (b.name.clone(), b)).collect();

  for b in &latest.blocks {
    let old_block = match old_blocks.get(&b.name) {
      Some(v) => v,
      None => &old.blocks[0], // Use air when we there is a missing block
    };
    for i in 0..b.states.len() as u32 {
      let old_id;
      if i as usize >= old_block.states.len() {
        old_id = 0;
      } else {
        old_id = old_block.states[i as usize].id;
      }
      to_old.push(old_id);
      to_new.push((old_id, to_old.len() as u32 - 1));
    }
  }

  (to_old, to_new)
}
