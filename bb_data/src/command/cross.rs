use super::CommandDef;
use crate::{gen::CodeGen, Version};
use std::collections::HashMap;

pub fn cross_version(gen: &mut CodeGen, old: &(Version, CommandDef), new: &(Version, CommandDef)) {
  let (old_ver, old_def) = old;
  let (_new_ver, new_def) = new;
  let (to_old, to_new) = find_ids(old_def, new_def);

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
    gen.write(&new_id.to_string());
  }
  gen.write_line("],");

  gen.write("ver: ");
  gen.write_line(&old_ver.to_block());

  gen.remove_indent();
  gen.write("}");
}

fn find_ids(old_def: &CommandDef, new_def: &CommandDef) -> (Vec<Option<u32>>, Vec<u32>) {
  let new_map: HashMap<_, _> = new_def.args.iter().map(|b| (b.name.clone(), b.clone())).collect();

  let mut to_new = Vec::with_capacity(new_def.args.len());
  for p in &old_def.args {
    to_new.push(new_map.get(&p.name).unwrap().id);
  }

  let mut to_old = Vec::with_capacity(to_new.len());
  for (old_id, new_id) in to_new.iter().enumerate() {
    let new_id = *new_id as usize;
    while to_old.len() <= new_id {
      to_old.push(None);
    }
    to_old[new_id] = Some(old_id as u32);
  }
  (to_old, to_new)
}
