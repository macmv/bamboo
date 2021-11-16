use super::{Packet, PacketDef};
use crate::{gen::CodeGen, Version};
use std::{collections::HashMap, fs, fs::File, io, io::Write, path::Path};

pub fn generate(def: Vec<(Version, PacketDef)>, dir: &Path) -> io::Result<()> {
  let mut all_cb_packets = HashMap::new();
  let mut all_sb_packets = HashMap::new();

  for (ver, def) in def {
    for p in def.clientbound {
      all_cb_packets.insert(p.name.clone(), (ver, p));
    }
    for p in def.serverbound {
      all_sb_packets.insert(p.name.clone(), (ver, p));
    }
  }

  let mut all_cb_packets: Vec<_> = all_cb_packets.into_iter().collect();
  all_cb_packets.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
  let mut all_sb_packets: Vec<_> = all_sb_packets.into_iter().collect();
  all_sb_packets.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

  fs::create_dir_all(dir)?;
  File::create(dir.join("cb.rs"))?.write_all(process(all_cb_packets).as_bytes())?;
  File::create(dir.join("sb.rs"))?.write_all(process(all_sb_packets).as_bytes())?;

  Ok(())
}

fn process(packets: Vec<(String, (Version, Packet))>) -> String {
  let mut gen = CodeGen::new();

  gen.write("pub enum Packet ");
  gen.write_block(|gen| {
    for (name, (ver, p)) in &packets {
      gen.write(name);
      gen.write_line(" {");
      gen.add_indent();
      for f in &p.fields {
        gen.write(&f.name);
        gen.write(": ");
        gen.write(&f.ty.to_rust());
        gen.write_line(",");
      }
      gen.remove_indent();
      gen.write_line("},");
    }
  });

  gen.into_output()
}
