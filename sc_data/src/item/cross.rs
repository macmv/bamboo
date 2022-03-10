use super::{Item, ItemDef};
use crate::{gen::CodeGen, Version};
use std::collections::HashMap;

pub fn cross_version(gen: &mut CodeGen, old: &(Version, ItemDef), new: &(Version, ItemDef)) {
  let (old_ver, old_def) = old;
  let (_new_ver, new_def) = new;
  let (to_old, to_new) = find_ids(*old_ver, old_def, new_def);

  gen.write_line("Version {");
  gen.add_indent();

  gen.write("to_old: &[");
  for (id, damage) in to_old {
    gen.write("(");
    gen.write(&id.to_string());
    gen.write(",");
    gen.write(&damage.to_string());
    gen.write("),");
  }
  gen.write_line("],");

  gen.write("to_new: &[");
  for ids in to_new {
    gen.write("&[");
    for id in ids {
      gen.write(&id.to_string());
      gen.write(",");
    }
    gen.write("],");
  }
  gen.write_line("],");

  gen.write("ver: ");
  gen.write_line(&old_ver.to_block());

  gen.remove_indent();
  gen.write("}");
}

fn find_ids(
  ver: Version,
  old_def: &ItemDef,
  new_def: &ItemDef,
) -> (Vec<(u32, u32)>, Vec<Vec<u32>>) {
  let mut old_def = old_def.clone();
  if old_def.items[0].name != "air" {
    old_def.items.insert(0, Item { id: 0, name: "air".into(), class: "".into() });
  }
  let old_map: HashMap<_, _> = old_def.items.iter().map(|b| (b.name.clone(), b.clone())).collect();

  let mut to_old = Vec::with_capacity(new_def.items.len());
  for i in &new_def.items {
    if ver.maj <= 12 {
      let (old_id, damage) = old_item(&i, &old_map);
      to_old.push((old_id, damage));
    } else {
      let old_item = old_map.get(&i.name).unwrap_or(&old_map["air"]);
      to_old.push((old_item.id, 0));
    }
  }

  let mut to_new = Vec::with_capacity(to_old.len());
  for (new_id, &(old_id, old_damage)) in to_old.iter().enumerate() {
    let old_id = old_id as usize;
    while to_new.len() <= old_id {
      to_new.push(vec![]);
    }
    while to_new[old_id].len() <= old_damage as usize {
      to_new[old_id].push(0);
    }
    to_new[old_id][old_damage as usize] = new_id as u32;
  }
  (to_old, to_new)
}

fn old_item(i: &Item, old_map: &HashMap<String, Item>) -> (u32, u32) {
  match i.name.as_str() {
    "granite" => (old_map["stone"].id, 1),
    "polished_granite" => (old_map["stone"].id, 2),
    "diorite" => (old_map["stone"].id, 3),
    "polished_diorite" => (old_map["stone"].id, 4),
    "andesite" => (old_map["stone"].id, 5),
    "polished_andesite" => (old_map["stone"].id, 6),

    "grass_block" => (old_map["grass"].id, 0),

    "coarse_dirt" => (old_map["dirt"].id, 1),
    "podzol" => (old_map["dirt"].id, 2),

    "oak_planks" => (old_map["planks"].id, 0),
    "spruce_planks" => (old_map["planks"].id, 1),
    "birch_planks" => (old_map["planks"].id, 2),
    "jungle_planks" => (old_map["planks"].id, 3),
    "acacia_planks" => (old_map["planks"].id, 4),
    "dark_oak_planks" => (old_map["planks"].id, 5),

    "oak_sapling" => (old_map["sapling"].id, 0),
    "spruce_sapling" => (old_map["sapling"].id, 1),
    "birch_sapling" => (old_map["sapling"].id, 2),
    "jungle_sapling" => (old_map["sapling"].id, 3),
    "acacia_sapling" => (old_map["sapling"].id, 4),
    "dark_oak_sapling" => (old_map["sapling"].id, 5),

    "red_sand" => (old_map["sand"].id, 1),

    "oak_log" => (old_map["log"].id, 0),
    "spruce_log" => (old_map["log"].id, 1),
    "birch_log" => (old_map["log"].id, 2),
    "jungle_log" => (old_map["log"].id, 3),
    "acacia_log" => (old_map["log"].id, 4),
    "dark_oak_log" => (old_map["log"].id, 5),

    "oak_leaves" => (old_map["leaves"].id, 0),
    "spruce_leaves" => (old_map["leaves"].id, 1),
    "birch_leaves" => (old_map["leaves"].id, 2),
    "jungle_leaves" => (old_map["leaves"].id, 3),
    "acacia_leaves" => (old_map["leaves"].id, 4),
    "dark_oak_leaves" => (old_map["leaves"].id, 5),

    "wet_sponge" => (old_map["sponge"].id, 1),

    "chiseled_sandstone" => (old_map["sandstone"].id, 1),
    "smooth_sandstone" => (old_map["sandstone"].id, 2),

    "dead_bush" => (old_map["tallgrass"].id, 0),
    "grass" => (old_map["tallgrass"].id, 1),
    "fern" => (old_map["tallgrass"].id, 2),

    "white_wool" => (old_map["wool"].id, 0),
    "orange_wool" => (old_map["wool"].id, 1),
    "magenta_wool" => (old_map["wool"].id, 2),
    "light_blue_wool" => (old_map["wool"].id, 3),
    "yellow_wool" => (old_map["wool"].id, 4),
    "lime_wool" => (old_map["wool"].id, 5),
    "pink_wool" => (old_map["wool"].id, 6),
    "gray_wool" => (old_map["wool"].id, 7),
    "light_gray_wool" => (old_map["wool"].id, 8),
    "cyan_wool" => (old_map["wool"].id, 9),
    "purple_wool" => (old_map["wool"].id, 10),
    "blue_wool" => (old_map["wool"].id, 11),
    "brown_wool" => (old_map["wool"].id, 12),
    "green_wool" => (old_map["wool"].id, 13),
    "red_wool" => (old_map["wool"].id, 14),
    "black_wool" => (old_map["wool"].id, 15),

    "dandelion" => (old_map["yellow_flower"].id, 0),
    "poppy" => (old_map["red_flower"].id, 0),
    "blue_orchid" => (old_map["red_flower"].id, 1),
    "allium" => (old_map["red_flower"].id, 2),
    "azure_bluet" => (old_map["red_flower"].id, 3),
    "red_tulip" => (old_map["red_flower"].id, 4),
    "orange_tulip" => (old_map["red_flower"].id, 5),
    "white_tulip" => (old_map["red_flower"].id, 6),
    "pink_tulip" => (old_map["red_flower"].id, 7),
    "oxeye_daisy" => (old_map["red_flower"].id, 8),

    "white_stained_glass_pane" => (old_map["stained_glass_pane"].id, 0),
    "orange_stained_glass_pane" => (old_map["stained_glass_pane"].id, 1),
    "magenta_stained_glass_pane" => (old_map["stained_glass_pane"].id, 2),
    "light_blue_stained_glass_pane" => (old_map["stained_glass_pane"].id, 3),
    "yellow_stained_glass_pane" => (old_map["stained_glass_pane"].id, 4),
    "lime_stained_glass_pane" => (old_map["stained_glass_pane"].id, 5),
    "pink_stained_glass_pane" => (old_map["stained_glass_pane"].id, 6),
    "gray_stained_glass_pane" => (old_map["stained_glass_pane"].id, 7),
    "light_gray_stained_glass_pane" => (old_map["stained_glass_pane"].id, 8),
    "cyan_stained_glass_pane" => (old_map["stained_glass_pane"].id, 9),
    "purple_stained_glass_pane" => (old_map["stained_glass_pane"].id, 10),
    "blue_stained_glass_pane" => (old_map["stained_glass_pane"].id, 11),
    "brown_stained_glass_pane" => (old_map["stained_glass_pane"].id, 12),
    "green_stained_glass_pane" => (old_map["stained_glass_pane"].id, 13),
    "red_stained_glass_pane" => (old_map["stained_glass_pane"].id, 14),
    "black_stained_glass_pane" => (old_map["stained_glass_pane"].id, 15),

    _ => (old_map.get(&i.name).unwrap_or(&old_map["air"]).id, 0),
  }
}

/*
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
    _ => old_map.get(&b.name).unwrap_or(&old_map["air"]).id,
  }
}
*/
