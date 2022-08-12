use super::EnchantmentDef;
use crate::{gen::CodeGen, Version};
use std::collections::HashMap;

pub fn cross_version(
  gen: &mut CodeGen,
  old: &(Version, EnchantmentDef),
  new: &(Version, EnchantmentDef),
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
  old_def: &EnchantmentDef,
  new_def: &EnchantmentDef,
) -> (Vec<Option<u32>>, Vec<Option<u32>>) {
  let new_map: HashMap<_, _> =
    new_def.enchantments.iter().map(|b| (b.name.clone(), b.clone())).collect();

  let mut to_new = Vec::with_capacity(new_def.enchantments.len());
  for e in &old_def.enchantments {
    let old_id = e.id as usize;
    while to_new.len() <= old_id {
      to_new.push(None);
    }
    to_new[old_id] =
      Some(if ver.maj <= 12 { new_map[&old_enchantment(&e.name)].id } else { new_map[&e.name].id });
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
fn old_enchantment(old: &str) -> String { old.into() }
