use crate::Collector;
use convert_case::{Case, Casing};
use serde::Deserialize;
use std::{collections::HashMap, fs, io};

mod cross;
mod gen;

pub fn generate(c: &Collector) -> io::Result<()> {
  fs::create_dir_all(c.out.join("entity"))?;
  let versions = crate::VERSIONS
    .iter()
    .map(|&ver| {
      let mut def: EntityDef = c.dl.get("entities", ver);
      def.init();
      (ver, def)
    })
    .collect();
  gen::generate(versions, &c.out.join("entity"))?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct EntityDef {
  entities:   Vec<Option<Entity>>,
  #[serde(skip)]
  entity_map: HashMap<String, usize>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Entity {
  /// The id of the entity.
  id:    u32,
  /// The name of the entity.
  name:  String,
  /// The full class of this entity.
  class: String,

  category:       String,
  width:          f32,
  height:         f32,
  tracking_range: u32,

  metadata: Vec<MetadataField>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetadataField {
  /// The index of this metadata field.
  id:   u32,
  /// The name of this field. This is how cross-versioning works. We use the
  /// yarn mappings here, and convert MCP mappings into yarn mappings.
  name: String,
  /// The kind of metadata.
  ty:   MetadataType,
}

// NBT is annoying, and I couldn't be bothered to fix it.
#[allow(clippy::upper_case_acronyms)]
/// An entity metadata type. Note that the documentation for this type is for
/// 1.18.2. Older versions will have different serializing/deserializing rules.
#[derive(Debug, Clone, Deserialize)]
pub enum MetadataType {
  /// A single byte.
  Byte,
  /// A varint (same as protocol).
  VarInt,
  /// A long. Only present on 1.19.3+
  Long,
  /// A short. Only present on 1.8-1.12.
  Short,
  /// A 4 byte floating point number
  Float,
  /// A varint prefixed string
  String,
  /// A string, which is JSON encoded chat data.
  Chat,
  /// A boolean. If true, this is followed by a Chat field.
  OptChat,
  /// An item stack. Same as protocol.
  Item,
  /// A single byte.
  Bool,
  /// 3 floats for X, Y, then Z.
  Rotation,
  /// A position encoded as a long.
  Position,
  /// A boolean. If true, this is followed by a Position.
  OptPosition,
  /// A VarInt. This will be from 0-5 (inclusive), which maps to a direction
  /// like so:
  /// - 0: Down
  /// - 1: Up
  /// - 2: North
  /// - 3: South
  /// - 4: West
  /// - 5: East
  Direction,
  /// A boolean. If true, then a 16 byte UUID follows.
  OptUUID,
  /// A varint, which should be parsed as a block ID.
  BlockID,
  /// An NBT tag. This is not length prefixed. The entire tag must be parsed to
  /// find the end of this field.
  NBT,
  /// A VarInt for the particle ID, followed by some data. The data following
  /// must be inferred from the particle ID.
  Particle,
  /// 3 VarInts: villager type, villager profession, and villager level.
  VillagerData,
  /// A boolean. If true, a VarInt follows.
  OptVarInt,
  /// A VarInt, from 0-7 (inclusive). The numbers map to these poses:
  /// - 0: Standing
  /// - 1: Fall flying
  /// - 2: Sleeping
  /// - 3: Swiming
  /// - 4: Spin attack
  /// - 5: Sneaking
  /// - 6: Long jumping
  /// - 7: Dying
  Pose,

  /// TODO: Figure out what this is!
  FireworkData,
  /// A varint
  CatVariant,
  /// A varint
  FrogVariant,
  /// A varint
  PaintingVariant,
}

impl EntityDef {
  pub fn init(&mut self) {
    for (id, ent) in self.entities.iter().enumerate() {
      if let Some(e) = ent {
        self.entity_map.insert(e.name.clone(), id);
      }
    }
  }

  /// Gets an entity. Unused, but might be useful in the future.
  #[allow(dead_code)]
  pub fn get(&self, name: &str) -> Option<&Entity> {
    let name = convert_name(name);
    if let Some(&idx) = self.entity_map.get(&name) {
      Some(self.entities[idx].as_ref().unwrap())
    } else {
      None
    }
  }
}

impl Entity {
  pub fn metadata_list(&self) -> Vec<Option<MetadataField>> {
    let mut sorted = self.metadata.clone();
    sorted.sort_unstable_by_key(|field| field.id);
    let mut list = Vec::with_capacity(sorted.len());
    let mut id = 0;
    for field in sorted {
      while id != field.id {
        list.push(None);
        id += 1;
      }
      list.push(Some(field));
      id += 1;
    }
    list
  }
}

// TODO: This converts all the old names to new ones, and it should actually be
// used.
#[allow(dead_code)]
fn convert_name(name: &str) -> String {
  let lower = name.to_case(Case::Snake);
  match lower.as_str() {
    // Can't exist, need a placeholder. TODO: Remove Mob from data collector.
    "mob" => "arrow",
    "monster" => "arrow",

    "xp_orb" => "experience_orb",
    "thrown_egg" => "egg",
    "thrown_enderpearl" => "ender_pearl",
    "thrown_potion" => "potion",
    "thrown_exp_bottle" | "xp_bottle" => "experience_bottle",
    "primed_tnt" => "tnt",
    "eye_of_ender_signal" => "eye_of_ender",
    "falling_sand" => "falling_block",
    "fireworks_rocket_entity" | "fireworks_rocket" => "firework_rocket",
    "minecart_command_block" | "commandblock_minecart" => "command_block_minecart",
    "minecart_rideable" => "minecart",
    "minecart_chest" => "chest_minecart",
    "minecart_furnace" => "furnace_minecart",
    "minecart_tnt" => "tnt_minecart",
    "minecart_hopper" => "hopper_minecart",
    "minecart_spawner" => "spawner_minecart",
    "pig_zombie" | "zombie_pigman" => "zombified_piglin",
    // ^^^^^^^^^   ^^^^^^^^^^^^^^^    ^^^^^^^^^^^^^^^^^^
    //   ????        correct name         stupid name
    "lava_slime" => "magma_cube",
    "wither_boss" => "wither",
    "mushroom_cow" => "mooshroom",
    // who changes their entity from `snow_man` to `snowman`
    "snow_man" | "snowman" => "snow_golem",
    "ozelot" => "ocelot", // spelling 100
    "villager_golem" => "iron_golem",
    "entity_horse" => "horse",
    "ender_crystal" => "end_crystal",

    "evocation_fangs" => "evoker_fangs",
    "evocation_illager" => "evoker",
    "vindication_illager" => "vindicator",
    "illusion_illager" => "illusioner",
    _ => return lower,
  }
  .into()
}
