use super::ParticleDef;
use crate::{gen::CodeGen, Version};
use std::collections::HashMap;

pub fn cross_version(
  gen: &mut CodeGen,
  old: &(Version, ParticleDef),
  new: &(Version, ParticleDef),
) {
  let (old_ver, old_def) = old;
  let (_new_ver, new_def) = new;
  let (to_old, to_new) = find_ids(*old_ver, old_def, new_def);

  gen.write_line("Version {");
  gen.add_indent();

  gen.write("to_old: &[");
  for old_id in to_old {
    match old_id {
      Some(id) => {
        gen.write("Some(");
        gen.write(&id.to_string());
        gen.write("),");
      }
      None => gen.write("None,"),
    }
  }
  gen.write_line("],");

  gen.write("to_new: &[");
  for new_id in to_new {
    match new_id {
      Some(id) => {
        gen.write("Some(");
        gen.write(&id.to_string());
        gen.write("),");
      }
      None => gen.write("None,"),
    }
  }
  gen.write_line("],");

  gen.write("ver: ");
  gen.write_line(&old_ver.to_block());

  gen.remove_indent();
  gen.write("}");
}

fn find_ids(
  ver: Version,
  old_def: &ParticleDef,
  new_def: &ParticleDef,
) -> (Vec<Option<u32>>, Vec<Option<u32>>) {
  let new_map: HashMap<_, _> =
    new_def.particles.iter().map(|b| (b.name.clone(), b.clone())).collect();

  let mut to_new = Vec::with_capacity(new_def.particles.len());
  for p in &old_def.particles {
    if ver.maj <= 12 {
      to_new.push(old_particle(&p.name).map(|new_name| new_map[&new_name].id));
    } else {
      to_new.push(new_map.get(&p.name).map(|p| p.id));
    }
  }

  let mut to_old = Vec::with_capacity(to_new.len());
  for (old_id, new_id) in to_new.iter().enumerate() {
    if let Some(new_id) = new_id {
      let new_id = *new_id as usize;
      while to_old.len() <= new_id {
        to_old.push(None);
      }
      to_old[new_id] = Some(old_id as u32);
    }
  }
  (to_old, to_new)
}

/// Maps an old name to a new one. This is backwards from other mappings, as not
/// every old id has a modern id. This could have been written the same way, but
/// it wasn't and I am not going to switch it.
fn old_particle(old: &str) -> Option<String> {
  let new = match old {
    "explode" => "poof",                          // very small explosion
    "largeexplode" => "explosion",                // normal explosion
    "hugeexplosion" => "explosion_emitter",       // large explosion
    "fireworksSpark" => "firework",               // firework trail
    "bubble" => "bubble",                         // light blue bubbles
    "splash" => "splash",                         // dark blue splashes
    "wake" => "fishing",                          // fishing bobber
    "suspended" => return None,                   // I cannot tell (the particle is too short)
    "depthsuspend" => "mycelium",                 // small gray particles that kinda hover
    "crit" => "crit",                             // shows when you crit an entity
    "magicCrit" => "enchanted_hit",               // blue particles shown when attacking armor
    "smoke" => "smoke",                           // black dust particles, which spread out
    "largesmoke" => "large_smoke",                // larger version of smoke
    "spell" => "effect",                          // spirals, used when you have an effect
    "instantSpell" => "instant_effect",           // crosses moving upward, used on splash potion
    "mobSpell" => "entity_effect",                // black spirals, seems the same as effect
    "mobSpellAmbient" => "ambient_entity_effect", // gray spirals, seems the same as effect
    "witchMagic" => "witch",                      // purple crosses
    "dripWater" => "dripping_water",              // blue circles stuck for a second, then falling
    "dripLava" => "dripping_lava",                // dripping_water but with orange circles
    "angryVillager" => "angry_villager",          // shown when you attack a villager
    "happyVillager" => "happy_villager",          // shown when you trade with a villager
    "townaura" => "mycelium",                     // appears to be the same as mycelium
    "note" => "note",                             // a musical note
    "portal" => "poral",                          // purple particles falling slowly
    "enchantmenttable" => "enchant",              // weird letters floating
    "flame" => "flame",                           // small fire icons
    "lava" => "lava",                             // orange embers with smoke flying out
    "footstep" => return None,                    // doens't exist on new versions
    "cloud" => "cloud",                           // similar to poof
    "reddust" => "dust",                          // redstone dust
    "snowballpoof" => "item_snowball",            // snowball collision particle
    "snowshovel" => "snowflake",                  // small snow particles falling
    "slime" => "item_slime",                      // same as item_snowball, but for slime
    "heart" => "heart",                           // red hearts, used after animals breed
    "barrier" => return None,                     // replaced with block_marker
    "iconcrack_" => return None,                  // doesn't render
    "blockcrack_" => "block",                     // block break particles
    "blockdust_" => "block",                      // particles which appear underfoot
    "droplet" => "rain",                          // rain splash on ground
    "take" => return None,                        // doens't render
    "mobappearance" => "elder_guardian",          // elder guardian appearing onscreen
    _ => return None,
  };
  Some(new.into())
}
