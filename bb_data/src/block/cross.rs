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
  /*
  let new_map: HashMap<_, _> = new_def.blocks.iter().map(|b| (b.name.clone(), b.clone())).collect();
  */

  let mut to_old = Vec::with_capacity(new_def.blocks.len());
  for b in &new_def.blocks {
    if ver.maj <= 12 {
      for state in b.all_states().iter() {
        let old_state = old_state(b, state, &old_map);
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

  let mut to_new = Vec::with_capacity(to_old.len());
  for (new_id, old_id) in to_old.iter().enumerate() {
    let old_id = *old_id as usize;
    while to_new.len() <= old_id {
      to_new.push(None);
    }
    // If the block id has already been set, we don't want to override it. This
    // means that when converting to a new id, we will always default to the lowest
    // id.
    if to_new[old_id].is_none() {
      to_new[old_id] = Some(new_id as u32);
    }
  }
  (to_old, to_new.into_iter().map(|v| v.unwrap_or(0)).collect())
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

// Adding `+ 0` makes this section look nice, so clippy can be ignored here.
#[allow(clippy::identity_op)]
fn old_state(b: &Block, state: &State, old_map: &HashMap<String, Block>) -> u32 {
  match b.name.as_str() {
    "granite" => old_map["stone"].id + 1,
    "polished_granite" => old_map["stone"].id + 2,
    "diorite" => old_map["stone"].id + 3,
    "polished_diorite" => old_map["stone"].id + 4,
    "andesite" => old_map["stone"].id + 5,
    "polished_andesite" => old_map["stone"].id + 6,

    "coarse_dirt" => old_map["dirt"].id + 1,
    "podzol" => old_map["dirt"].id + 2,

    "oak_planks" => old_map["planks"].id + 0,
    "spruce_planks" => old_map["planks"].id + 1,
    "birch_planks" => old_map["planks"].id + 2,
    "jungle_planks" => old_map["planks"].id + 3,
    "acacia_planks" => old_map["planks"].id + 4,
    "dark_oak_planks" => old_map["planks"].id + 5,

    "oak_sapling" => old_map["sapling"].id + 0,
    "spruce_sapling" => old_map["sapling"].id + 1,
    "birch_sapling" => old_map["sapling"].id + 2,
    "jungle_sapling" => old_map["sapling"].id + 3,
    "acacia_sapling" => old_map["sapling"].id + 4,
    "dark_oak_sapling" => old_map["sapling"].id + 5,

    "water" => match state.int_prop("level") {
      0 => old_map["water"].id,
      // Only levels 1 through 7 are valid. 8 through 15 produce a full water section, which
      // dissapears after a liquid update. This happens in every version from 1.8-1.18. It is
      // unclear why this property spans from 0 to 15, but it does.
      level @ 1..=15 => old_map["flowing_water"].id + level as u32 - 1,
      _ => unreachable!(),
    },
    "lava" => match state.int_prop("level") {
      0 => old_map["lava"].id,
      // Same thing with flowing as water
      level @ 1..=15 => old_map["flowing_lava"].id + level as u32 - 1,
      _ => unreachable!(),
    },

    "red_sand" => old_map["sand"].id + 1,

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
    "oak_wood" => old_map["log"].id + 12 + 0,
    "spruce_wood" => old_map["log"].id + 12 + 1,
    "birch_wood" => old_map["log"].id + 12 + 2,
    "jungle_wood" => old_map["log"].id + 12 + 3,

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

    "wet_sponge" => old_map["sponge"].id + 1,

    "dispenser" => match state.enum_prop("facing") {
      "DOWN" => old_map["dispenser"].id + 0,
      "UP" => old_map["dispenser"].id + 1,
      "NORTH" => old_map["dispenser"].id + 2,
      "SOUTH" => old_map["dispenser"].id + 3,
      "WEST" => old_map["dispenser"].id + 4,
      "EAST" => old_map["dispenser"].id + 5,
      _ => unreachable!(),
    },

    "chiseled_sandstone" => old_map["sandstone"].id + 1,
    "smooth_sandstone" => old_map["sandstone"].id + 2,

    "white_wool" => old_map["wool"].id + 0,
    "orange_wool" => old_map["wool"].id + 1,
    "magenta_wool" => old_map["wool"].id + 2,
    "light_blue_wool" => old_map["wool"].id + 3,
    "yellow_wool" => old_map["wool"].id + 4,
    "lime_wool" => old_map["wool"].id + 5,
    "pink_wool" => old_map["wool"].id + 6,
    "gray_wool" => old_map["wool"].id + 7,
    "light_gray_wool" => old_map["wool"].id + 8,
    "cyan_wool" => old_map["wool"].id + 9,
    "purple_wool" => old_map["wool"].id + 10,
    "blue_wool" => old_map["wool"].id + 11,
    "brown_wool" => old_map["wool"].id + 12,
    "green_wool" => old_map["wool"].id + 13,
    "red_wool" => old_map["wool"].id + 14,
    "black_wool" => old_map["wool"].id + 15,

    "dandelion" => old_map["yellow_flower"].id,
    "poppy" => old_map["red_flower"].id + 0,
    "blue_orchid" => old_map["red_flower"].id + 1,
    "allium" => old_map["red_flower"].id + 2,
    "azure_bluet" => old_map["red_flower"].id + 3,
    "red_tulip" => old_map["red_flower"].id + 4,
    "orange_tulip" => old_map["red_flower"].id + 5,
    "white_tulip" => old_map["red_flower"].id + 6,
    "pink_tulip" => old_map["red_flower"].id + 7,
    "oxeye_daisy" => old_map["red_flower"].id + 8,

    "sandstone_slab" => old_map["stone_slab"].id + 1,
    "oak_slab" => old_map["stone_slab"].id + 2,
    "cobblestone_slab" => old_map["stone_slab"].id + 3,
    "brick_slab" => old_map["stone_slab"].id + 4,
    "stone_brick_slab" => old_map["stone_slab"].id + 5,

    // MINECRAFT GO BRRRRRR
    "grass_block" => old_map["grass"].id,
    "grass" => old_map["tallgrass"].id + 1,

    "dead_bush" => old_map["tallgrass"].id + 0,
    "fern" => old_map["tallgrass"].id + 2,

    "oak_door" => {
      match (
        state.bool_prop("powered"),
        state.enum_prop("hinge"),
        state.enum_prop("half"),
        state.bool_prop("open"),
        state.enum_prop("facing"),
      ) {
        (_, _, "LOWER", false, "EAST") => old_map["wooden_door"].id + 0,
        (_, _, "LOWER", false, "SOUTH") => old_map["wooden_door"].id + 1,
        (_, _, "LOWER", false, "WEST") => old_map["wooden_door"].id + 2,
        (_, _, "LOWER", false, "NORTH") => old_map["wooden_door"].id + 3,
        (_, _, "LOWER", true, "EAST") => old_map["wooden_door"].id + 4 + 0,
        (_, _, "LOWER", true, "SOUTH") => old_map["wooden_door"].id + 4 + 1,
        (_, _, "LOWER", true, "WEST") => old_map["wooden_door"].id + 4 + 2,
        (_, _, "LOWER", true, "NORTH") => old_map["wooden_door"].id + 4 + 3,
        (false, "LEFT", "UPPER", _, _) => old_map["wooden_door"].id + 8,
        (false, "RIGHT", "UPPER", _, _) => old_map["wooden_door"].id + 9,
        (true, "LEFT", "UPPER", _, _) => old_map["wooden_door"].id + 10,
        (true, "RIGHT", "UPPER", _, _) => old_map["wooden_door"].id + 11,
        _ => unreachable!("invalid state {state:?}"),
      }
    }
    _ => old_map.get(&b.name).unwrap_or(&old_map["air"]).id,
  }
}
