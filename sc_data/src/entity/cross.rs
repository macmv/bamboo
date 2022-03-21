use super::{Entity, EntityDef};
use crate::{gen::CodeGen, Version};
use convert_case::{Case, Casing};
use std::collections::HashMap;

pub fn cross_version_metadata(
  gen: &mut CodeGen,
  old_ver: &Version,
  old: &EntityDef,
  new: &EntityDef,
  to_old: &[u32],
) {
  gen.write_line("metadata: &[");
  gen.add_indent();
  for ent in &new.entities {
    let new = ent.as_ref().unwrap();
    let old = &old.entities[to_old[new.id as usize] as usize];
    if let Some(old) = old {
      let (to_old, to_new) = find_metadata_ids(old, new);
      gen.write_comment(&new.name);
      gen.write_line("Metadata {");
      gen.add_indent();

      gen.write("to_old: &[");
      for id in &to_old {
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

      gen.write_line("old_types: &[");
      gen.add_indent();
      for field in &old.metadata_list() {
        if let Some(field) = field {
          gen.write("Some(MetadataType::");
          gen.write(&format!("{:?}", field.ty));
          gen.write_line("),");
        } else {
          gen.write_line("None,");
        }
      }
      gen.remove_indent();
      gen.write_line("],");

      gen.write_line("new_types: &[");
      gen.add_indent();
      for field in &new.metadata {
        gen.write("MetadataType::");
        gen.write(&format!("{:?}", field.ty));
        gen.write_line(",");
      }
      gen.remove_indent();
      gen.write_line("],");

      gen.remove_indent();
      gen.write_line("},");
    }
  }
  gen.remove_indent();
  gen.write_line("],");
}

fn find_metadata_ids(old_def: &Entity, new_def: &Entity) -> (Vec<u32>, Vec<u32>) {
  let old_map: HashMap<_, _> =
    old_def.metadata.iter().map(|b| (b.name.clone(), b.clone())).collect();

  let mut to_old = Vec::with_capacity(new_def.metadata.len());
  let mut max_old_id = 0;
  for e in &new_def.metadata {
    let name = e.name.clone();
    match old_map.get(&name) {
      Some(old_meta) => {
        if old_meta.id > max_old_id {
          max_old_id = old_meta.id;
        }
        to_old.push(old_meta.id)
      }
      None => to_old.push(0),
    };
  }

  let mut to_new = vec![None; max_old_id as usize + 1];
  for (new_id, old_id) in to_old.iter().enumerate() {
    let old_id = *old_id as usize;
    // If the block id has already been set, we don't want to override it. This
    // means that when converting to a new id, we will always default to the lowest
    // id.
    if to_new[old_id].is_none() {
      to_new[old_id] = Some(new_id as u32);
    }
  }
  (to_old, to_new.into_iter().map(|v| v.unwrap_or(0)).collect())
}

pub fn cross_version(gen: &mut CodeGen, old: &(Version, EntityDef), new: &(Version, EntityDef)) {
  let (old_ver, old_def) = old;
  let (_new_ver, new_def) = new;
  let (to_old, to_new) = find_ids(*old_ver, old_def, new_def);

  gen.write_line("Version {");
  gen.add_indent();

  gen.write("to_old: &[");
  for id in &to_old {
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

  cross_version_metadata(gen, old_ver, old_def, new_def, &to_old);

  gen.write("ver: ");
  gen.write_line(&old_ver.to_block());

  gen.remove_indent();
  gen.write("}");
}

fn find_ids(ver: Version, old_def: &EntityDef, new_def: &EntityDef) -> (Vec<u32>, Vec<u32>) {
  let old_def = old_def.clone();
  let old_map: HashMap<_, _> = old_def
    .entities
    .iter()
    .flat_map(|b| Some((b.as_ref()?.name.clone(), b.as_ref()?.clone())))
    .collect();

  let mut to_old = Vec::with_capacity(new_def.entities.len());
  for e in &new_def.entities {
    let e = e.as_ref().unwrap();
    let name = if ver.maj <= 10 { convert_name(&e.name) } else { e.name.clone() };
    match old_map.get(&name) {
      Some(old_entity) => to_old.push(old_entity.id),
      None => to_old.push(0),
    };
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

fn convert_name(name: &str) -> String { name.to_case(Case::Pascal) }

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
