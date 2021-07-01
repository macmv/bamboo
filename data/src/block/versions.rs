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
      if l.is_empty() {
        return vec![];
      }
      // This is the new block and old block
      let sections: Vec<&str> = l.split(':').map(|s| s.trim()).collect();
      // This is the old block name and optional metadata
      let right_sections: Vec<&str> = sections[1].split(' ').collect();
      let mut values = vec![];
      let old_name = if right_sections.len() == 1 {
        (right_sections[0], 0)
      } else {
        (right_sections[0], right_sections[1].parse().unwrap())
      };
      if sections[0].contains("{color}") {
        values.push((sections[0].replace("{color}", "white"), old_name));
        values.push((sections[0].replace("{color}", "orange"), old_name));
        values.push((sections[0].replace("{color}", "magenta"), old_name));
        values.push((sections[0].replace("{color}", "light_blue"), old_name));
        values.push((sections[0].replace("{color}", "yellow"), old_name));
        values.push((sections[0].replace("{color}", "lime"), old_name));
        values.push((sections[0].replace("{color}", "pink"), old_name));
        values.push((sections[0].replace("{color}", "gray"), old_name));
        values.push((sections[0].replace("{color}", "light_gray"), old_name));
        values.push((sections[0].replace("{color}", "cyan"), old_name));
        values.push((sections[0].replace("{color}", "purple"), old_name));
        values.push((sections[0].replace("{color}", "blue"), old_name));
        values.push((sections[0].replace("{color}", "brown"), old_name));
        values.push((sections[0].replace("{color}", "green"), old_name));
        values.push((sections[0].replace("{color}", "red"), old_name));
        values.push((sections[0].replace("{color}", "black"), old_name));
      } else {
        values.push((sections[0].into(), old_name))
      }
      values
    })
    .flatten()
    .collect();

  let mut all_blocks: HashMap<String, &Block> = HashMap::new();
  for (names, b) in latest.blocks.iter().map(|b| (b.prop_strs(), b)) {
    for n in names {
      all_blocks.insert(n, b);
    }
  }
  all_blocks
    .extend(latest.blocks.iter().map(|b| (b.name.clone(), b)).collect::<HashMap<String, &Block>>());
  for n in names.keys() {
    if !all_blocks.contains_key(n) {
      let sections: Vec<&str> = n.split('[').collect();
      if sections.len() == 1 {
        panic!("invalid modern block name: {}", n);
      }
      match all_blocks.get(sections[0]) {
        Some(b) => {
          panic!("invalid modern block name: {}. block strs are: {:?}", n, b.prop_strs())
        }
        None => panic!("invalid modern block name: {}", n),
      }
    }
  }

  let old_blocks: HashMap<String, Block> =
    old.blocks.iter().cloned().map(|b| (b.name.clone(), b)).collect();

  for b in &latest.blocks {
    // First lookup by prop string. If it fails, then lookup by block name.
    // Otherwise, just use this block name.
    for s in b.prop_strs() {
      let (old_name, old_meta) = match names.get(&s) {
        Some(v) => *v,
        None => match names.get(&b.name) {
          Some(v) => *v,
          None => (b.name.as_ref(), 0),
        },
      };
      let old_block = match old_blocks.get(old_name) {
        Some(v) => v,
        None => {
          println!("warning: missing block {}", old_name);
          &old.blocks[0] // Use air when we there is a missing block
        }
      };
      to_old.push(old_block.id | old_meta);
    }
  }

  to_old
}
