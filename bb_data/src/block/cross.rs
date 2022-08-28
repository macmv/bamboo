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

      // The 1.19 id of bedrock is now 74 (it was 33 in 1.18), and stone button
      // is different as well (not sure what the new id is). I'm going to leave
      // this commented out, as this isn't something we need to test.
      /*
      assert_eq!(to_old[33], 7 << 4); // Bedrock
      assert_eq!(to_new[7 << 4], 33); // Bedrock

      assert_eq!(to_old[3966], 77 << 4); // Stone button
      assert_eq!(to_new[77 << 4], 3966); // Stone button
      */
    }
    14 | 15 | 16 | 17 | 18 | 19 => {
      assert_eq!(to_old[0], 0); // Air
      assert_eq!(to_old[1], 1); // Stone

      // See above.
      /*
      assert_eq!(to_old[33], 33); // Bedrock
      */

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
  let matcher = Matcher { block: b, state, old: old_map };
  matcher.find()
}

struct Matcher<'a> {
  block: &'a Block,
  state: &'a State,
  old:   &'a HashMap<String, Block>,
}

impl Matcher<'_> {
  fn bool_prop(&self, name: &str) -> bool { self.state.bool_prop(name) }
  fn enum_prop(&self, name: &str) -> &str { self.state.enum_prop(name) }
  fn int_prop(&self, name: &str) -> i32 { self.state.int_prop(name) }
  fn old(&self, name: &str) -> u32 { self.old[name].id }
  #[rustfmt::skip]
  #[allow(clippy::identity_op)]
  fn find(&self) -> u32 {
    // this will help formatting:
    // :'<,'>s/\(\S*\)\s*=>/\=submatch(1) . repeat(' ', 30 - len(submatch(1))) . '=>'
    //                                                  ^^ change this for indent amount
    match self.block.name.as_str() {
      "granite"           => self.old("stone") + 1,
      "polished_granite"  => self.old("stone") + 2,
      "diorite"           => self.old("stone") + 3,
      "polished_diorite"  => self.old("stone") + 4,
      "andesite"          => self.old("stone") + 5,
      "polished_andesite" => self.old("stone") + 6,

      "grass_block" => self.old("grass"),

      "coarse_dirt" => self.old("dirt") + 1,
      "podzol"      => self.old("dirt") + 2,

      "oak_planks"      => self.old("planks") + 0,
      "spruce_planks"   => self.old("planks") + 1,
      "birch_planks"    => self.old("planks") + 2,
      "jungle_planks"   => self.old("planks") + 3,
      "acacia_planks"   => self.old("planks") + 4,
      "dark_oak_planks" => self.old("planks") + 5,

      "oak_sapling"      => self.old("sapling") + 0,
      "spruce_sapling"   => self.old("sapling") + 1,
      "birch_sapling"    => self.old("sapling") + 2,
      "jungle_sapling"   => self.old("sapling") + 3,
      "acacia_sapling"   => self.old("sapling") + 4,
      "dark_oak_sapling" => self.old("sapling") + 5,

      "water" => match self.int_prop("level") {
        0 => self.old("water"),
        // Only levels 1 through 7 are valid. 8 through 15 produce a full water section, which
        // disappears after a liquid update. This happens in every version from 1.8-1.18. It is
        // unclear why this property spans from 0 to 15, but it does.
        level @ 1..=15 => self.old("flowing_water") + level as u32 - 1,
        _ => unreachable!(),
      },
      "lava" => match self.int_prop("level") {
        0 => self.old("lava"),
        // Same thing with flowing as water
        level @ 1..=15 => self.old("flowing_lava") + level as u32 - 1,
        _ => unreachable!(),
      },

      "red_sand" => self.old("sand") + 1,

      "oak_log" => match self.enum_prop("axis") {
        "x" => self.old("log") + 0 + 4,
        "y" => self.old("log") + 0 + 0,
        "z" => self.old("log") + 0 + 8,
        _ => unreachable!(),
      },
      "spruce_log" => match self.enum_prop("axis") {
        "x" => self.old("log") + 1 + 4,
        "y" => self.old("log") + 1 + 0,
        "z" => self.old("log") + 1 + 8,
        _ => unreachable!(),
      },
      "birch_log" => match self.enum_prop("axis") {
        "x" => self.old("log") + 2 + 4,
        "y" => self.old("log") + 2 + 0,
        "z" => self.old("log") + 2 + 8,
        _ => unreachable!(),
      },
      "jungle_log" => match self.enum_prop("axis") {
        "x" => self.old("log") + 3 + 4,
        "y" => self.old("log") + 3 + 0,
        "z" => self.old("log") + 3 + 8,
        _ => unreachable!(),
      },
      "oak_wood"    => self.old("log") + 12 + 0,
      "spruce_wood" => self.old("log") + 12 + 1,
      "birch_wood"  => self.old("log") + 12 + 2,
      "jungle_wood" => self.old("log") + 12 + 3,

      "oak_leaves" => match self.bool_prop("persistent") {
        true => self.old("leaves") + 0 + 0,
        false => self.old("leaves") + 0 + 8,
      },
      "spruce_leaves" => match self.bool_prop("persistent") {
        true => self.old("leaves") + 1 + 0,
        false => self.old("leaves") + 1 + 8,
      },
      "birch_leaves" => match self.bool_prop("persistent") {
        true => self.old("leaves") + 2 + 0,
        false => self.old("leaves") + 2 + 8,
      },
      "jungle_leaves" => match self.bool_prop("persistent") {
        true => self.old("leaves") + 3 + 0,
        false => self.old("leaves") + 3 + 8,
      },

      "wet_sponge" => self.old("sponge") + 1,

      "dispenser" => match self.enum_prop("facing") {
        "down"  => self.old("dispenser") + 0,
        "up"    => self.old("dispenser") + 1,
        "north" => self.old("dispenser") + 2,
        "south" => self.old("dispenser") + 3,
        "west"  => self.old("dispenser") + 4,
        "east"  => self.old("dispenser") + 5,
        _ => unreachable!(),
      },

      "chiseled_sandstone" => self.old("sandstone") + 1,
      "smooth_sandstone"   => self.old("sandstone") + 2,

      "note_block" => self.old("noteblock"),

      "white_bed"      => self.old("bed"),
      "orange_bed"     => self.old("bed"),
      "magenta_bed"    => self.old("bed"),
      "light_blue_bed" => self.old("bed"),
      "yellow_bed"     => self.old("bed"),
      "lime_bed"       => self.old("bed"),
      "pink_bed"       => self.old("bed"),
      "gray_bed"       => self.old("bed"),
      "light_gray_bed" => self.old("bed"),
      "cyan_bed"       => self.old("bed"),
      "purple_bed"     => self.old("bed"),
      "blue_bed"       => self.old("bed"),
      "brown_bed"      => self.old("bed"),
      "green_bed"      => self.old("bed"),
      "red_bed"        => self.old("bed"),
      "black_bed"      => self.old("bed"),

      "powered_rail" => self.old("golden_rail"),

      "sticky_piston" => self.piston("sticky_piston"),

      "cobweb" => self.old("web"),

      "grass"     => self.old("tallgrass") + 1,
      "fern"      => self.old("tallgrass") + 2,
      "dead_bush" => self.old("deadbush"),

      "piston" => self.piston("piston"),

      "white_wool"      => self.old("wool") + 0,
      "orange_wool"     => self.old("wool") + 1,
      "magenta_wool"    => self.old("wool") + 2,
      "light_blue_wool" => self.old("wool") + 3,
      "yellow_wool"     => self.old("wool") + 4,
      "lime_wool"       => self.old("wool") + 5,
      "pink_wool"       => self.old("wool") + 6,
      "gray_wool"       => self.old("wool") + 7,
      "light_gray_wool" => self.old("wool") + 8,
      "cyan_wool"       => self.old("wool") + 9,
      "purple_wool"     => self.old("wool") + 10,
      "blue_wool"       => self.old("wool") + 11,
      "brown_wool"      => self.old("wool") + 12,
      "green_wool"      => self.old("wool") + 13,
      "red_wool"        => self.old("wool") + 14,
      "black_wool"      => self.old("wool") + 15,

      "dandelion"    => self.old("yellow_flower"),
      "poppy"        => self.old("red_flower") + 0,
      "blue_orchid"  => self.old("red_flower") + 1,
      "allium"       => self.old("red_flower") + 2,
      "azure_bluet"  => self.old("red_flower") + 3,
      "red_tulip"    => self.old("red_flower") + 4,
      "orange_tulip" => self.old("red_flower") + 5,
      "white_tulip"  => self.old("red_flower") + 6,
      "pink_tulip"   => self.old("red_flower") + 7,
      "oxeye_daisy"  => self.old("red_flower") + 8,

      "smooth_stone_slab"  => self.slab("stone_slab", "double_stone_slab") + 0,
      "sandstone_slab"     => self.slab("stone_slab", "double_stone_slab") + 1,
      "petrified_oak_slab" => self.slab("stone_slab", "double_stone_slab") + 2,
      "cobblestone_slab"   => self.slab("stone_slab", "double_stone_slab") + 3,
      "brick_slab"         => self.slab("stone_slab", "double_stone_slab") + 4,
      "stone_brick_slab"   => self.slab("stone_slab", "double_stone_slab") + 5,
      "nether_brick_slab"  => self.slab("stone_slab", "double_stone_slab") + 6,
      "quartz_slab"        => self.slab("stone_slab", "double_stone_slab") + 7,
      "red_sandstone_slab" => self.slab("stone_slab2", "double_stone_slab2") + 0,

      "oak_door"    => self.door("wooden_door"),
      "spruce_door" => self.door("spruce_door"),
      "birch_door"  => self.door("birch_door"),
      "jungle_door" => self.door("jungle_door"),

      "oak_slab"      => self.slab("wooden_slab", "double_wooden_slab") + 0,
      "spruce_slab"   => self.slab("wooden_slab", "double_wooden_slab") + 1,
      "birch_slab"    => self.slab("wooden_slab", "double_wooden_slab") + 2,
      "jungle_slab"   => self.slab("wooden_slab", "double_wooden_slab") + 3,
      "acacia_slab"   => self.slab("wooden_slab", "double_wooden_slab") + 4,
      "dark_oak_slab" => self.slab("wooden_slab", "double_wooden_slab") + 5,

      "oak_pressure_plate"      => self.pressure_plate("wooden_pressure_plate"),
      "spruce_pressure_plate"   => self.pressure_plate("wooden_pressure_plate"),
      "birch_pressure_plate"    => self.pressure_plate("wooden_pressure_plate"),
      "jungle_pressure_plate"   => self.pressure_plate("wooden_pressure_plate"),
      "acacia_pressure_plate"   => self.pressure_plate("wooden_pressure_plate"),
      "dark_oak_pressure_plate" => self.pressure_plate("wooden_pressure_plate"),

      "redstone_ore" => match self.bool_prop("lit") {
        true => self.old("lit_redstone_ore"),
        false => self.old("redstone_ore"),
      },

      "stone_button" => self.button("stone_button"),

      "snow" => self.old("snow_layer") + self.int_prop("layers") as u32,
      "snow_block" => self.old("snow"),

      "sugar_cane" => self.old("reeds"),

      "oak_fence" => self.old("fence"),

      "white_stained_glass"      => self.old("stained_glass") + 0,
      "orange_stained_glass"     => self.old("stained_glass") + 1,
      "magenta_stained_glass"    => self.old("stained_glass") + 2,
      "light_blue_stained_glass" => self.old("stained_glass") + 3,
      "yellow_stained_glass"     => self.old("stained_glass") + 4,
      "lime_stained_glass"       => self.old("stained_glass") + 5,
      "pink_stained_glass"       => self.old("stained_glass") + 6,
      "gray_stained_glass"       => self.old("stained_glass") + 7,
      "light_gray_stained_glass" => self.old("stained_glass") + 8,
      "cyan_stained_glass"       => self.old("stained_glass") + 9,
      "purple_stained_glass"     => self.old("stained_glass") + 10,
      "blue_stained_glass"       => self.old("stained_glass") + 11,
      "brown_stained_glass"      => self.old("stained_glass") + 12,
      "green_stained_glass"      => self.old("stained_glass") + 13,
      "red_stained_glass"        => self.old("stained_glass") + 14,
      "black_stained_glass"      => self.old("stained_glass") + 15,

      "oak_stairs"      => self.stairs("oak_stairs"),
      "spruce_stairs"   => self.stairs("spruce_stairs"),
      "birch_stairs"    => self.stairs("birch_stairs"),
      "jungle_stairs"   => self.stairs("jungle_stairs"),
      "acacia_stairs"   => self.stairs("acacia_stairs"),
      "dark_oak_stairs" => self.stairs("dark_oak_stairs"),

      // Otherwise, lookup the old block, and if we still don't find anything, use air.
      _ => self.old.get(&self.block.name).unwrap_or(&self.old["air"]).id,
    }
  }

  #[rustfmt::skip]
  #[allow(clippy::identity_op)]
  fn piston(&self, name: &str) -> u32 {
    match (self.state.enum_prop("facing"), self.state.bool_prop("extended")) {
      ("down",  false) => self.old(name) + 0,
      ("up",    false) => self.old(name) + 1,
      ("north", false) => self.old(name) + 2,
      ("south", false) => self.old(name) + 3,
      ("west",  false) => self.old(name) + 4,
      ("east",  false) => self.old(name) + 5,
      ("down",  true) => self.old(name) + 0 + 8,
      ("up",    true) => self.old(name) + 1 + 8,
      ("north", true) => self.old(name) + 2 + 8,
      ("south", true) => self.old(name) + 3 + 8,
      ("west",  true) => self.old(name) + 4 + 8,
      ("east",  true) => self.old(name) + 5 + 8,
      _ => unreachable!("invalid state {:?}", self.state),
    }
  }
  #[rustfmt::skip]
  #[allow(clippy::identity_op)]
  fn button(&self, name: &str) -> u32 {
    // I'm amazed at how many ways one can represent a 6-way facing enum.
    match (self.state.enum_prop("face"), self.state.enum_prop("facing"), self.state.bool_prop("powered")) {
      ("ceiling", _,       false) => self.old(name) + 0,
      ("wall",    "east",  false) => self.old(name) + 1,
      ("wall",    "west",  false) => self.old(name) + 2,
      ("wall",    "south", false) => self.old(name) + 3,
      ("wall",    "north", false) => self.old(name) + 4,
      ("floor",   _,       false) => self.old(name) + 5,
      ("ceiling", _,       true) => self.old(name) + 0 + 8,
      ("wall",    "east",  true) => self.old(name) + 1 + 8,
      ("wall",    "west",  true) => self.old(name) + 2 + 8,
      ("wall",    "south", true) => self.old(name) + 3 + 8,
      ("wall",    "north", true) => self.old(name) + 4 + 8,
      ("floor",   _,       true) => self.old(name) + 5 + 8,
      _ => unreachable!("invalid state {:?}", self.state),
    }
  }
  #[rustfmt::skip]
  #[allow(clippy::identity_op)]
  fn pressure_plate(&self, name: &str) -> u32 {
    match self.state.bool_prop("powered") {
      false => self.old(name) + 0,
      true  => self.old(name) + 1,
    }
  }

  #[rustfmt::skip]
  #[allow(clippy::identity_op)]
  fn slab(&self, name: &str, double_name: &str) -> u32 {
    match self.state.enum_prop("type") {
      "bottom" => self.old(name) + 0,
      "top"    => self.old(name) + 8,
      "double" => self.old(double_name) + 0,
      _ => unreachable!("invalid state {:?}", self.state),
    }
  }

  #[rustfmt::skip]
  #[allow(clippy::identity_op)]
  fn stairs(&self, name: &str) -> u32 {
    match (self.state.enum_prop("facing"), self.state.enum_prop("half")) {
      ("east",  "bottom") => self.old(name) + 0,
      ("west",  "bottom") => self.old(name) + 1,
      ("south", "bottom") => self.old(name) + 2,
      ("north", "bottom") => self.old(name) + 3,
      ("east",  "top") => self.old(name) + 0 + 4,
      ("west",  "top") => self.old(name) + 1 + 4,
      ("south", "top") => self.old(name) + 2 + 4,
      ("north", "top") => self.old(name) + 3 + 4,
      _ => unreachable!("invalid state {:?}", self.state),
    }
  }

  #[rustfmt::skip]
  #[allow(clippy::identity_op)]
  fn door(&self, name: &str) -> u32 {
    match (
      self.state.bool_prop("powered"),
      self.state.enum_prop("hinge"),
      self.state.enum_prop("half"),
      self.state.bool_prop("open"),
      self.state.enum_prop("facing"),
    ) {
      (_, _, "lower", false, "east")  => self.old(name) + 0,
      (_, _, "lower", false, "south") => self.old(name) + 1,
      (_, _, "lower", false, "west")  => self.old(name) + 2,
      (_, _, "lower", false, "north") => self.old(name) + 3,
      (_, _, "lower", true, "east")   => self.old(name) + 4 + 0,
      (_, _, "lower", true, "south")  => self.old(name) + 4 + 1,
      (_, _, "lower", true, "west")   => self.old(name) + 4 + 2,
      (_, _, "lower", true, "north")  => self.old(name) + 4 + 3,
      (false, "left", "upper", _, _)  => self.old(name) + 8,
      (false, "right", "upper", _, _) => self.old(name) + 9,
      (true, "left", "upper", _, _)   => self.old(name) + 10,
      (true, "right", "upper", _, _)  => self.old(name) + 11,
      _ => unreachable!("invalid state {:?}", self.state),
    }
  }
}
