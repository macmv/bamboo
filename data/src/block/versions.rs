use super::{Block, BlockVersion};
use std::collections::HashMap;

// Called on 1.13+
pub(super) fn generate(latest: &BlockVersion, old: &BlockVersion) -> Vec<u32> {
  let mut to_old = vec![];

  let old_blocks: HashMap<String, Block> =
    old.blocks.iter().cloned().map(|b| (b.name.clone(), b)).collect();

  for b in &latest.blocks {
    let old_block = match old_blocks.get(&b.name) {
      Some(v) => v,
      None => &old.blocks[0], // Use air when we there is a missing block
    };
    if b.states.is_empty() {
      to_old.push(old_block.id);
    } else {
      for s in &b.states {
        let mut old_id = 0;
        for o in &old_block.states {
          if o.properties == s.properties {
            old_id = o.id;
            break;
          }
        }
        to_old.push(old_id);
      }
    }
  }

  to_old
}

// Called on 1.8-1.12
pub(super) fn generate_old(latest: &BlockVersion, old: &BlockVersion) -> Vec<u32> {
  let mut to_old = vec![];

  // Map of new block names to old block names and metadata values
  let names: HashMap<String, (&str, u32)> = include_str!("old_names.txt")
    .trim()
    .split('\n')
    .map(|l| {
      let sections: Vec<&str> = l.split(": ").collect();
      let sub_sections: Vec<&str> = sections[1].split(' ').collect();
      if sub_sections.len() == 2 {
        (sections[0].into(), (sub_sections[0], sub_sections[1].parse().unwrap()))
      } else {
        (sections[0].into(), (sub_sections[0], 0))
      }
    })
    .collect();

  let old_blocks: HashMap<String, Block> =
    old.blocks.iter().cloned().map(|b| (b.name.clone(), b)).collect();

  for b in &latest.blocks {
    let (old_name, old_meta) = *names.get(&b.name).unwrap_or(&(&b.name, 0));
    let old_block = match old_blocks.get(old_name) {
      Some(v) => v,
      None => {
        println!("warning: missing block {}", old_name);
        &old.blocks[0] // Use air when we there is a missing block
      }
    };
    if b.states.is_empty() {
      to_old.push(old_block.id | old_meta);
    } else {
      for s in &b.states {
        let mut old_id = 0;
        for o in &old_block.states {
          if o.properties == s.properties {
            old_id = o.id;
            break;
          }
        }
        to_old.push(old_id);
      }
    }
  }

  to_old
}
