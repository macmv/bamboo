use super::{Block, BlockVersion};
use std::collections::HashMap;

// Called on 1.13+
pub(super) fn generate(latest: &BlockVersion, old: &BlockVersion) -> Vec<u32> {
  let mut to_old = vec![];

  let old_blocks: HashMap<String, Block> =
    old.blocks.iter().cloned().map(|b| (b.name().to_string(), b)).collect();

  for b in &latest.blocks {
    let old_block = match old_blocks.get(b.name()) {
      Some(v) => v,
      None => &old.blocks[0], // Use air when we there is a missing block
    };
    if b.states().is_empty() {
      to_old.push(old_block.id());
    } else {
      for s in b.states() {
        let mut old_id = 0;
        for o in old_block.states() {
          if o.props() == s.props() {
            old_id = o.id();
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

  // The keys in names are prop strings, so we need to convert them into States
  let mut state_maps: HashMap<String, Vec<(Vec<(String, String)>, String, u32)>> = HashMap::new();
  for (key, (old_name, old_meta)) in names {
    let mut iter = key.split('[');
    let name = iter.next().unwrap().to_string();
    let mut props = HashMap::new();
    if let Some(mut prop_str) = iter.next() {
      prop_str = prop_str.split(']').next().unwrap();
      for pair in prop_str.split(',') {
        let mut iter = pair.split('=');
        let key = iter.next().unwrap();
        let val = iter.next().unwrap();

        props.insert(key.to_string(), val.to_string());
      }
    }
    // Props is an incomplete list of properties for the block. We want to say
    // that all possible blocks with those properties have the given old
    // block name/id let key = State { id: blocks[name], properties };

    if let Some(values) = state_maps.get_mut(&name) {
      values.push((props.into_iter().collect(), old_name.into(), old_meta));
    } else {
      state_maps.insert(name, vec![(props.into_iter().collect(), old_name.into(), old_meta)]);
    }
  }

  // Need to iterate in order
  for b in &latest.blocks {
    match state_maps.get(b.name()) {
      Some(old_values) => {
        if b.states().is_empty() {
          to_old.push(0);
        } else {
          for state in b.states() {
            let mut old_id = 0;
            for (props, old_name, old_meta) in old_values {
              if state.matches(&props) {
                old_id = old.get(old_name).id() | old_meta;
                break;
              }
            }
            to_old.push(old_id);
          }
        }
        // for (state, matches) in b.matching_states(props) {}
        // for state in &b.states {
        //   let mut matched = false;
        //   'outer: for (key, val) in &state.properties {
        //     for (other_key, other_val) in props {
        //       if key == other_key && val == other_val {
        //         // We want to convert `state` into an old block
        //         to_old.push(old.get(old_name).id | old_meta);
        //         matched = true;
        //         break 'outer;
        //       }
        //     }
        //   }
        //   if !matched {
        //     to_old.push(0);
        //   }
        // }
      }
      None => {
        if b.states().is_empty() {
          to_old.push(0);
        } else {
          for _ in b.states() {
            to_old.push(0);
          }
        }
      }
    }
  }

  // let mut all_blocks: HashMap<String, &Block> = HashMap::new();
  // for (names, b) in latest.blocks.iter().map(|b| (b.prop_strs(), b)) {
  //   for n in names {
  //     all_blocks.insert(n, b);
  //   }
  // }
  // all_blocks
  //   .extend(latest.blocks.iter().map(|b| (b.name.clone(),
  // b)).collect::<HashMap<String, &Block>>()); for n in names.keys() {
  //   if !all_blocks.contains_key(n) {
  //     let sections: Vec<&str> = n.split('[').collect();
  //     if sections.len() == 1 {
  //       panic!("invalid modern block name: {}", n);
  //     }
  //     match all_blocks.get(sections[0]) {
  //       Some(b) => {
  //         panic!("invalid modern block name: {}. block strs are: {:?}", n,
  // b.prop_strs())       }
  //       None => panic!("invalid modern block name: {}", n),
  //     }
  //   }
  // }

  // let old_blocks: HashMap<String, Block> =
  //   old.blocks.iter().cloned().map(|b| (b.name.clone(), b)).collect();
  //
  // for b in &latest.blocks {
  //   // First lookup by prop string. If it fails, then lookup by block name.
  //   // Otherwise, just use this block name.
  //   for s in b.prop_strs() {
  //     let (old_name, old_meta) = match names.get(&s) {
  //       Some(v) => *v,
  //       None => match names.get(&b.name) {
  //         Some(v) => *v,
  //         None => (b.name.as_ref(), 0),
  //       },
  //     };
  //     let old_block = match old_blocks.get(old_name) {
  //       Some(v) => v,
  //       None => {
  //         println!("warning: missing block {}", old_name);
  //         &old.blocks[0] // Use air when we there is a missing block
  //       }
  //     };
  //     to_old.push(old_block.id | old_meta);
  //   }
  // }
  //
  to_old
}
