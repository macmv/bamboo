use super::{BlockDef, Prop, PropKind};
use crate::{gen::CodeGen, Version};
use std::collections::HashMap;

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
    let old_block = old_map.get(&b.name).unwrap_or(&old_map["air"]);
    for (_sid, _) in b.all_states().iter().enumerate() {
      // let sid = sid as u32;
      to_old.push(old_block.id);
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
    convert_old_name(&mut b.name);
    // Old block ids are weird. In chunk data, they are left shifted by 4. The new
    // empty 4 bits are used for the 16 state ids. This means that if we want to do
    // state conversions correctly, we need to shift this over.
    b.id <<= 4;
    b.properties = vec![Prop { name: "id".into(), kind: PropKind::Int { min: 0, max: 16 } }];
  }
}

fn convert_old_name(name: &mut String) {
  let new = match name.as_str() {
    "grass" => "grass_block",
    _ => return,
  };
  *name = new.into();
}
