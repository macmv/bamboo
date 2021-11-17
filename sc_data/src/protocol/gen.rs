use super::{Packet, PacketDef};
use crate::{
  gen::{CodeGen, MatchBranch},
  Version,
};
use std::{collections::HashMap, fs, fs::File, io, io::Write, path::Path};

pub fn generate(def: Vec<(Version, PacketDef)>, dir: &Path) -> io::Result<()> {
  let mut all_cb_packets = PacketCollection::new();
  let mut all_sb_packets = PacketCollection::new();

  for (ver, def) in def {
    for p in def.clientbound {
      all_cb_packets.add(ver, p);
    }
    for p in def.serverbound {
      all_sb_packets.add(ver, p);
    }
  }

  fs::create_dir_all(dir)?;
  File::create(dir.join("cb.rs"))?.write_all(all_cb_packets.gen().as_bytes())?;
  File::create(dir.join("sb.rs"))?.write_all(all_sb_packets.gen().as_bytes())?;

  Ok(())
}
struct PacketCollection {
  packets: HashMap<String, Vec<(Version, Packet)>>,
}

impl PacketCollection {
  pub fn new() -> Self {
    PacketCollection { packets: HashMap::new() }
  }
  pub fn add(&mut self, ver: Version, p: Packet) {
    let list = self.packets.entry(p.name.clone()).or_insert_with(|| vec![]);
    if let Some((_, last)) = list.last() {
      if *last == p {
        return;
      }
    }
    list.push((ver, p));
  }
  pub fn gen(self) -> String {
    let mut gen = CodeGen::new();

    let mut packets: Vec<_> = self.packets.into_iter().collect();
    packets.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    let packets: Vec<Vec<(_, _)>> = packets.into_iter().map(|(_, v)| v).collect();

    gen.write("pub enum Packet ");
    gen.write_block(|gen| {
      for versions in &packets {
        for (ver, p) in versions {
          write_packet(gen, &format!("{}V{}", p.name, ver.maj), p);
        }
      }
    });

    gen.write_impl("Packet", |gen| {
      gen.write("pub fn from_tcp(p: tcp::Packet, ver: ProtocolVersion) -> Self ");
      gen.write_block(|gen| {
        gen.write_match("to_sug_id(p.id(), ver)", |gen| {
          for (id, versions) in packets.iter().enumerate() {
            gen.write(&id.to_string());
            gen.write(" => ");
            gen.write_block(|gen| {
              let (ver, first) = versions.first().unwrap();
              if ver.maj != 8 {
                gen.write("if ver < ");
                gen.write(&ver.maj.to_string());
                gen.write(" ");
                gen.write_block(|gen| {
                  gen.write(r#"panic!("version {} is below the minimum version for packet "#);
                  gen.write(&first.name);
                  gen.write_line(r#" ");"#);
                });
              }
              for (i, (_, p)) in versions.iter().enumerate() {
                if let Some(next_ver) = versions.get(i + 1) {
                  gen.write("if ver < ");
                  gen.write(&next_ver.0.maj.to_string());
                  gen.write_line(" {");
                  gen.add_indent();
                  write_from_tcp(gen, p);
                  gen.remove_indent();
                  gen.write("} else ");
                } else {
                  gen.write_block(|gen| {
                    write_from_tcp(gen, p);
                  });
                }
              }
            });
          }
        });
      });
    });

    gen.into_output()
  }
}

fn write_packet(gen: &mut CodeGen, name: &str, p: &Packet) {
  gen.write(name);
  gen.write_line(" {");
  gen.add_indent();
  for f in &p.fields {
    if f.name == "type" {
      gen.write("ty");
    } else {
      gen.write(&f.name);
    }
    gen.write(": ");
    gen.write(&f.ty.to_rust());
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("},");
}

fn write_from_tcp(gen: &mut CodeGen, p: &Packet) {
  gen.write_line("p.read_varint()");
}
