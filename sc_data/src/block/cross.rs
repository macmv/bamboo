use super::{Block, BlockDef, Prop, PropKind, PropValue, State};
use crate::{gen::CodeGen, Version};
use std::collections::HashMap;

#[cfg(test)]
pub fn cross_test(old: &(Version, BlockDef), new: &(Version, BlockDef)) {
  let (old_ver, old_def) = old;
  let (_new_ver, new_def) = new;
  let (to_old, to_new) = find_ids(*old_ver, old_def, new_def);

  match old_ver.maj {
    8 | 9 | 10 | 11 | 12 => {
      assert_eq!(to_old[0], 0); // Air
      assert_eq!(to_new[0], 0); // Air
      assert_eq!(to_old[1], 1 << 4); // Stone
      assert_eq!(to_new[1 << 4], 1); // Stone
      assert_eq!(to_old[33], 7 << 4); // Bedrock
      assert_eq!(to_new[7 << 4], 33); // Bedrock

      assert_eq!(to_old[3966], 77 << 4); // Stone button
      assert_eq!(to_new[77 << 4], 3966); // Stone button
    }
    14 | 15 | 16 | 17 | 18 => {
      assert_eq!(to_old[0], 0); // Air
      assert_eq!(to_old[1], 1); // Stone
      assert_eq!(to_old[33], 33); // Bedrock

      // The two variants of grass
      assert_eq!(to_old[8], 8);
      assert_eq!(to_old[9], 9);
    }
    _ => {
      panic!("unknown version {}", old_ver);
    }
  }
}

pub fn cross_version(gen: &mut CodeGen, old: &(Version, BlockDef), new: &(Version, BlockDef)) {
  let (old_ver, old_def) = old;
  let (_new_ver, new_def) = new;
  let (to_old, to_new) = find_ids(*old_ver, old_def, new_def);

  gen.write_line("Version {");
  gen.add_indent();

  gen.write("to_old: &[");
  for id in to_old {
    gen.write(&id.to_string());
    gen.write(",");
  }
  gen.write_line("],");

  gen.write("to_new: &[");
  for id in to_new {
    gen.write(&id.to_string());
    gen.write(",");
  }
  gen.write_line("],");

  gen.write("ver: ");
  gen.write_line(&old_ver.to_block());

  gen.remove_indent();
  gen.write("}");
}

fn find_ids(ver: Version, old_def: &BlockDef, new_def: &BlockDef) -> (Vec<u32>, Vec<u32>) {
  let mut old_def = old_def.clone();
  if ver.maj <= 12 {
    update_old_blocks(&mut old_def);
  }

  let old_map: HashMap<_, _> = old_def.blocks.iter().map(|b| (b.name.clone(), b.clone())).collect();
  let new_map: HashMap<_, _> = new_def.blocks.iter().map(|b| (b.name.clone(), b.clone())).collect();

  let mut to_old = Vec::with_capacity(new_def.blocks.len());
  for b in &new_def.blocks {
    if ver.maj <= 12 {
      for (sid, state) in b.all_states().iter().enumerate() {
        let sid = sid as u32;
        let old_state = old_state(&b, sid, state, &old_map);
        to_old.push(old_state);
      }
    } else {
      let old_block = old_map.get(&b.name).unwrap_or(&old_map["air"]);
      if old_block.all_states().len() == b.all_states().len() {
        // If we have the same number of states, the properties are probably the same,
        // so we just want to copy it directly.
        for (sid, _) in b.all_states().iter().enumerate() {
          to_old.push(old_block.id + sid as u32);
        }
      } else {
        // TODO: If the number of states differ, then we should do some property
        // comparison here.
        for _ in b.all_states().iter() {
          to_old.push(old_block.id);
        }
      }
    }
  }

  let mut to_new = Vec::with_capacity(old_def.blocks.len());
  for b in &old_def.blocks {
    let new_block = new_map.get(&b.name).unwrap_or(&new_map["air"]);
    for (sid, _) in b.all_states().iter().enumerate() {
      let sid = sid as u32;
      to_new.push(new_block.id + sid);
    }
  }
  (to_old, to_new)
}

fn update_old_blocks(def: &mut BlockDef) {
  for b in &mut def.blocks {
    // Old block ids are weird. In chunk data, they are left shifted by 4. The new
    // empty 4 bits are used for the 16 state ids. This means that if we want to do
    // state conversions correctly, we need to shift this over.
    b.id <<= 4;
    b.properties = vec![Prop {
      name:    "id".into(),
      kind:    PropKind::Int { min: 0, max: 16 },
      default: PropValue::Int(0),
    }];
  }
}

fn old_state(b: &Block, sid: u32, state: &State, old_map: &HashMap<String, Block>) -> u32 {
  match b.name.as_str() {
    "oak_log" => match state.enum_prop("axis") {
      "X" => old_map["log"].id + 0 + 4,
      "Y" => old_map["log"].id + 0 + 0,
      "Z" => old_map["log"].id + 0 + 8,
      _ => unreachable!(),
    },
    "spruce_log" => match state.enum_prop("axis") {
      "X" => old_map["log"].id + 1 + 4,
      "Y" => old_map["log"].id + 1 + 0,
      "Z" => old_map["log"].id + 1 + 8,
      _ => unreachable!(),
    },
    "birch_log" => match state.enum_prop("axis") {
      "X" => old_map["log"].id + 2 + 4,
      "Y" => old_map["log"].id + 2 + 0,
      "Z" => old_map["log"].id + 2 + 8,
      _ => unreachable!(),
    },
    "jungle_log" => match state.enum_prop("axis") {
      "X" => old_map["log"].id + 3 + 4,
      "Y" => old_map["log"].id + 3 + 0,
      "Z" => old_map["log"].id + 3 + 8,
      _ => unreachable!(),
    },
    "oak_wood" => old_map["log"].id + 4 + 0,
    "spruce_wood" => old_map["log"].id + 4 + 1,
    "birch_wood" => old_map["log"].id + 4 + 2,
    "jungle_wood" => old_map["log"].id + 4 + 3,

    "oak_leaves" => match state.bool_prop("persistent") {
      true => old_map["leaves"].id + 0 + 0,
      false => old_map["leaves"].id + 0 + 8,
    },
    "spruce_leaves" => match state.bool_prop("persistent") {
      true => old_map["leaves"].id + 1 + 0,
      false => old_map["leaves"].id + 1 + 8,
    },
    "birch_leaves" => match state.bool_prop("persistent") {
      true => old_map["leaves"].id + 2 + 0,
      false => old_map["leaves"].id + 2 + 8,
    },
    "jungle_leaves" => match state.bool_prop("persistent") {
      true => old_map["leaves"].id + 3 + 0,
      false => old_map["leaves"].id + 3 + 8,
    },

    // MINECRAFT GO BRRRRRR
    "grass_block" => old_map["grass"].id,
    "grass" => old_map["tallgrass"].id + 1,

    "dead_bush" => old_map["tallgrass"].id + 0,
    "fern" => old_map["tallgrass"].id + 2,
    _ => old_map.get(&b.name).unwrap_or(&old_map["air"]).id + sid,
  }
}
