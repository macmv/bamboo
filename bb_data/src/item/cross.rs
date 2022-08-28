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
      let (old_id, damage) = old_item(i, &old_map);
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

struct OldItem {
  id:   u32,
  meta: u32,
}

impl OldItem {
  pub fn new(id: u32) -> Self { OldItem { id, meta: 0 } }
  pub fn meta(mut self, meta: u32) -> Self {
    self.meta = meta;
    self
  }
}
fn old_item(i: &Item, old_map: &HashMap<String, Item>) -> (u32, u32) {
  let matcher = Matcher { item: i, old: old_map };
  let old = matcher.find();
  (old.id, old.meta)
}
struct Matcher<'a> {
  item: &'a Item,
  old:  &'a HashMap<String, Item>,
}
impl Matcher<'_> {
  fn old(&self, name: &str) -> OldItem { OldItem::new(self.old[name].id) }
  #[rustfmt::skip]
  #[allow(clippy::identity_op)]
  fn find(&self) -> OldItem {
    match self.item.name.as_str() {
      "granite"           => self.old("stone").meta(1),
      "polished_granite"  => self.old("stone").meta(2),
      "polished_diorite"  => self.old("stone").meta(4),
      "diorite"           => self.old("stone").meta(3),
      "andesite"          => self.old("stone").meta(5),
      "polished_andesite" => self.old("stone").meta(6),

      "grass_block" => self.old("grass"),

      "coarse_dirt" => self.old("dirt").meta(1),
      "podzol"      => self.old("dirt").meta(2),

      "oak_planks"      => self.old("planks").meta(0),
      "spruce_planks"   => self.old("planks").meta(1),
      "birch_planks"    => self.old("planks").meta(2),
      "jungle_planks"   => self.old("planks").meta(3),
      "acacia_planks"   => self.old("planks").meta(4),
      "dark_oak_planks" => self.old("planks").meta(5),

      "oak_sapling"      => self.old("sapling").meta(0),
      "spruce_sapling"   => self.old("sapling").meta(1),
      "birch_sapling"    => self.old("sapling").meta(2),
      "jungle_sapling"   => self.old("sapling").meta(3),
      "acacia_sapling"   => self.old("sapling").meta(4),
      "dark_oak_sapling" => self.old("sapling").meta(5),

      "red_sand" => self.old("sand").meta(1),

      "oak_log"      => self.old("log").meta(0),
      "spruce_log"   => self.old("log").meta(1),
      "birch_log"    => self.old("log").meta(2),
      "jungle_log"   => self.old("log").meta(3),
      "acacia_log"   => self.old("log").meta(4),
      "dark_oak_log" => self.old("log").meta(5),

      "oak_leaves"      => self.old("leaves").meta(0),
      "spruce_leaves"   => self.old("leaves").meta(1),
      "birch_leaves"    => self.old("leaves").meta(2),
      "jungle_leaves"   => self.old("leaves").meta(3),
      "acacia_leaves"   => self.old("leaves").meta(4),
      "dark_oak_leaves" => self.old("leaves").meta(5),

      "wet_sponge" => self.old("sponge").meta(1),

      "chiseled_sandstone" => self.old("sandstone").meta(1),
      "smooth_sandstone"   => self.old("sandstone").meta(2),

      "dead_bush" => self.old("tallgrass").meta(0),
      "grass"     => self.old("tallgrass").meta(1),
      "fern"      => self.old("tallgrass").meta(2),

      "white_wool"      => self.old("wool").meta(0),
      "orange_wool"     => self.old("wool").meta(1),
      "magenta_wool"    => self.old("wool").meta(2),
      "light_blue_wool" => self.old("wool").meta(3),
      "yellow_wool"     => self.old("wool").meta(4),
      "lime_wool"       => self.old("wool").meta(5),
      "pink_wool"       => self.old("wool").meta(6),
      "gray_wool"       => self.old("wool").meta(7),
      "light_gray_wool" => self.old("wool").meta(8),
      "cyan_wool"       => self.old("wool").meta(9),
      "purple_wool"     => self.old("wool").meta(10),
      "blue_wool"       => self.old("wool").meta(11),
      "brown_wool"      => self.old("wool").meta(12),
      "green_wool"      => self.old("wool").meta(13),
      "red_wool"        => self.old("wool").meta(14),
      "black_wool"      => self.old("wool").meta(15),

      "dandelion"    => self.old("yellow_flower").meta(0),
      "poppy"        => self.old("red_flower").meta(0),
      "blue_orchid"  => self.old("red_flower").meta(1),
      "allium"       => self.old("red_flower").meta(2),
      "azure_bluet"  => self.old("red_flower").meta(3),
      "red_tulip"    => self.old("red_flower").meta(4),
      "orange_tulip" => self.old("red_flower").meta(5),
      "white_tulip"  => self.old("red_flower").meta(6),
      "pink_tulip"   => self.old("red_flower").meta(7),
      "oxeye_daisy"  => self.old("red_flower").meta(8),

      "white_stained_glass_pane"      => self.old("stained_glass_pane").meta(0),
      "orange_stained_glass_pane"     => self.old("stained_glass_pane").meta(1),
      "magenta_stained_glass_pane"    => self.old("stained_glass_pane").meta(2),
      "light_blue_stained_glass_pane" => self.old("stained_glass_pane").meta(3),
      "yellow_stained_glass_pane"     => self.old("stained_glass_pane").meta(4),
      "lime_stained_glass_pane"       => self.old("stained_glass_pane").meta(5),
      "pink_stained_glass_pane"       => self.old("stained_glass_pane").meta(6),
      "gray_stained_glass_pane"       => self.old("stained_glass_pane").meta(7),
      "light_gray_stained_glass_pane" => self.old("stained_glass_pane").meta(8),
      "cyan_stained_glass_pane"       => self.old("stained_glass_pane").meta(9),
      "purple_stained_glass_pane"     => self.old("stained_glass_pane").meta(10),
      "blue_stained_glass_pane"       => self.old("stained_glass_pane").meta(11),
      "brown_stained_glass_pane"      => self.old("stained_glass_pane").meta(12),
      "green_stained_glass_pane"      => self.old("stained_glass_pane").meta(13),
      "red_stained_glass_pane"        => self.old("stained_glass_pane").meta(14),
      "black_stained_glass_pane"      => self.old("stained_glass_pane").meta(15),

      "smooth_stone_slab"  => self.old("stone_slab").meta(0),
      "sandstone_slab"     => self.old("stone_slab").meta(1),
      "petrified_oak_slab" => self.old("stone_slab").meta(2),
      "cobblestone_slab"   => self.old("stone_slab").meta(3),
      "brick_slab"         => self.old("stone_slab").meta(4),
      "stone_brick_slab"   => self.old("stone_slab").meta(5),
      "nether_brick_slab"  => self.old("stone_slab").meta(6),
      "quartz_slab"        => self.old("stone_slab").meta(7),
      "red_sandstone_slab" => self.old("stone_slab2").meta(0),

      "oak_slab"      => self.old("wooden_slab").meta(0),
      "spruce_slab"   => self.old("wooden_slab").meta(1),
      "birch_slab"    => self.old("wooden_slab").meta(2),
      "jungle_slab"   => self.old("wooden_slab").meta(3),
      "acacia_slab"   => self.old("wooden_slab").meta(4),
      "dark_oak_slab" => self.old("wooden_slab").meta(5),

      // Special case. We add some enchantments to a normal stick in the proxy,
      // so that it looks like a debug stick.
      "debug_stick" => self.old("stick").meta(1),

      _ => OldItem::new(self.old.get(&self.item.name).map(|it| it.id).unwrap_or(0)),
    }
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
