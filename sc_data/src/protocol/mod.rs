mod field;
mod gen;
mod json;
mod parse;

pub use field::{
  BitField, Container, CountType, FloatType, IntType, NamedPacketField, PacketField, VersionedField,
};

use crate::gen::{AppendIters, CodeGen, EnumVariant, FuncArg, MatchBranch};
use convert_case::{Case, Casing};
use itertools::Itertools;
use serde_derive::{Deserialize, Serialize};
use std::{
  cmp,
  collections::{HashMap, HashSet},
  error::Error,
  fmt, fs,
  path::Path,
  str::FromStr,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
  pub name:        String,
  // Can be used to lookup a field by name
  pub field_names: HashMap<String, usize>,
  pub fields:      Vec<(String, PacketField)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketVersion {
  // The index is the packet's id. The names should be mapped to the indicies as well.
  pub types:     HashMap<String, PacketField>,
  pub to_client: Vec<Packet>,
  pub to_server: Vec<Packet>,
}

#[derive(Debug)]
struct VersionedPacket {
  name:        String,
  field_names: HashMap<String, usize>,
  // Map of versions to fields
  fields:      Vec<VersionedField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
  major: i32,
  minor: i32,
}

impl cmp::PartialOrd for Version {
  fn partial_cmp(&self, other: &Version) -> Option<cmp::Ordering> {
    Some(self.cmp(other))
  }
}
impl cmp::Ord for Version {
  fn cmp(&self, other: &Version) -> cmp::Ordering {
    if self.major == other.major {
      self.minor.cmp(&other.minor)
    } else {
      self.major.cmp(&other.major)
    }
  }
}
impl fmt::Display for Version {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.minor == 0 {
      write!(f, "v1_{}", self.major)
    } else {
      write!(f, "v1_{}_{}", self.major, self.minor)
    }
  }
}
#[derive(Debug)]
pub struct VersionErr(String);
impl fmt::Display for VersionErr {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid version {}", self.0)
  }
}
impl Error for VersionErr {}

impl FromStr for Version {
  type Err = VersionErr;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut sections = s.split("_");
    let first = sections.next().ok_or_else(|| VersionErr(s.to_string()))?;
    let major = sections.next().ok_or_else(|| VersionErr(s.to_string()))?;
    let minor = sections.next();
    if sections.next() != None {
      return Err(VersionErr(s.to_string()));
    }
    if first != "V1" {
      return Err(VersionErr(s.to_string()));
    }
    let major = major.parse().map_err(|_| VersionErr(s.to_string()))?;
    let minor = minor.map(|s| s.parse()).unwrap_or(Ok(0)).map_err(|_| VersionErr(s.to_string()))?;
    Ok(Version { major, minor })
  }
}

impl VersionedPacket {
  fn new(name: String) -> Self {
    VersionedPacket { name, field_names: HashMap::new(), fields: vec![] }
  }

  fn add_version(&mut self, ver: Version, packet: Packet) {
    let mut missing_fields: HashSet<String> = self.field_names.keys().cloned().collect();
    // This is the index into `self.fields`. It is updated whenever we find a valid
    // packet field in the new packet. This ensures that removed fields will not
    // offset indices of fields later in the set.
    //
    // For example, if we have 3 fields:
    //
    // - A
    // - B
    // - C
    //
    // Then add another version which removes A, and addes D:
    //
    // - B
    // - C
    // - D
    //
    // If we used enumerate(), the index for D would be wrong. This is because we
    // still need to hold onto A, so D ends up being the fourth item, not the third
    // item.
    let mut idx = 0;
    for (name, field) in packet.fields.into_iter() {
      if let Some(&i) = self.field_names.get(&name) {
        missing_fields.remove(&name);
        let existing = &mut self.fields[i];
        if existing.latest() != &field {
          existing.add_ver(ver, field);
        }
        idx = i;
      } else {
        // We need to update all the field_names mappings before hand.
        for (_, val) in self.field_names.iter_mut() {
          if *val >= idx {
            *val += 1;
          }
        }
        self.field_names.insert(name.clone(), idx);
        // If we add a field from a later version, we need to make sure the fields stay
        // in order
        self.fields.insert(idx, VersionedField::new(ver, name, field));
      }
      idx += 1;
    }
    for name in missing_fields {
      let idx = self.field_names[&name];
      self.fields[idx].set_removed_version(ver);
    }
  }

  fn fields(&self) -> Vec<NamedPacketField> {
    let mut out = vec![];
    for field in &self.fields {
      field.add_all(&mut out);
    }
    out
  }

  fn fields_ver(&self, ver: Version) -> Vec<(bool, NamedPacketField)> {
    let mut out = vec![];
    for field in &self.fields {
      field.add_all_ver(&mut out, ver);
    }
    out
  }

  fn has_multiple_versions(&self) -> bool {
    for field in &self.fields {
      if field.multi_versioned() {
        return true;
      }
    }
    false
  }
  fn all_versions(&self) -> Vec<Version> {
    let mut versions = HashSet::new();
    for field in &self.fields {
      if field.multi_versioned() {
        for (ver, _) in &field.versions {
          versions.insert(ver.clone());
        }
      }
    }
    versions.into_iter().sorted().collect()
  }
  fn name(&self) -> &str {
    &self.name
  }
}

impl NamedPacketField {
  fn name(&self) -> String {
    self.name.clone()
  }
  fn ty(&self) -> String {
    if self.multi_versioned {
      format!("Option<{}>", self.field.ty_lit())
    } else {
      self.field.ty_lit().to_string()
    }
  }
  fn write_to_proto(&self, gen: &mut CodeGen) {
    gen.write_line(&format!("{}?;", self.generate_to_proto()));
  }
  fn write_from_proto(&self, gen: &mut CodeGen, is_ver: bool) {
    gen.write(&self.name());
    gen.write(": ");
    if self.multi_versioned {
      if is_ver {
        gen.write("Some(");
        gen.write(self.generate_from_proto());
        gen.write(")");
      } else {
        gen.write("None");
      }
    } else {
      gen.write(self.generate_from_proto());
    }
    gen.write_line(",");
  }
  fn write_from_tcp(&self, gen: &mut CodeGen, is_ver: bool) {
    gen.write(&self.name());
    gen.write(": ");
    if self.multi_versioned {
      if is_ver {
        gen.write("Some(");
        gen.write(self.generate_from_tcp());
        gen.write(")");
      } else {
        gen.write("None");
      }
    } else {
      gen.write(self.generate_from_tcp());
    }
    gen.write_line(",");
  }
  fn generate_to_proto(&self) -> String {
    if self.multi_versioned {
      self.field.generate_to_sc(&format!("{}.as_ref().unwrap()", self.name))
    } else {
      self.field.generate_to_sc(&self.name)
    }
  }
  fn generate_from_proto(&self) -> &'static str {
    self.field.generate_from_sc()
  }
  fn generate_to_tcp(&self) -> String {
    if self.multi_versioned {
      self.field.generate_to_tcp(&format!("{}.as_ref().unwrap()", self.name))
    } else {
      self.field.generate_to_tcp(&self.name)
    }
  }
  fn generate_from_tcp(&self) -> &'static str {
    self.field.generate_from_tcp()
  }
}

fn to_versioned(
  versions: &HashMap<Version, PacketVersion>,
) -> (Vec<VersionedPacket>, Vec<VersionedPacket>) {
  // Generates the packet id enum, for clientbound and serverbound packets
  let mut to_client = HashMap::new();
  let mut to_server = HashMap::new();

  for (version, v) in versions.iter().sorted_by(|(ver_a, _), (ver_b, _)| ver_a.cmp(ver_b)) {
    for p in &v.to_client {
      if !to_client.contains_key(&p.name) {
        to_client.insert(p.name.clone(), VersionedPacket::new(p.name.clone()));
      }
      to_client.get_mut(&p.name).unwrap().add_version(*version, p.clone());
    }
    for p in &v.to_server {
      if !to_server.contains_key(&p.name) {
        to_server.insert(p.name.clone(), VersionedPacket::new(p.name.clone()));
      }
      to_server.get_mut(&p.name).unwrap().add_version(*version, p.clone());
    }
    if !to_server.contains_key("Login") {
      to_server.insert("Login".into(), VersionedPacket::new("Login".into()));
    }
    to_server.get_mut("Login").unwrap().add_version(
      *version,
      Packet {
        name:        "Login".into(),
        field_names: [("username", 0), ("uuid", 1), ("ver", 2)]
          .iter()
          .cloned()
          .map(|(k, v)| (k.to_string(), v))
          .collect(),
        fields:      vec![
          ("username".into(), PacketField::String),
          ("uuid".into(), PacketField::UUID),
          ("ver".into(), PacketField::Int(IntType::I32)),
        ],
      },
    );
  }
  // This is a custom packet. It is a packet sent from the proxy to the server,
  // which is used to authenticate the player.

  let to_client: Vec<VersionedPacket> = to_client
    .into_iter()
    .sorted_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b))
    .map(|(_, packet)| packet)
    .collect();
  let to_server: Vec<VersionedPacket> = to_server
    .into_iter()
    .sorted_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b))
    .map(|(_, packet)| packet)
    .collect();

  (to_client, to_server)
}

pub fn generate(dir: &Path) -> Result<(), Box<dyn Error>> {
  let prismarine_path = dir.join("prismarine-data");
  let dir = dir.join("protocol");

  // This is done at runtime of the buildscript, so this path must be relative to
  // where the buildscript is.
  let versions = parse::load_all(&prismarine_path.join("data/pc"))?
    .into_iter()
    .map(|(ver, val)| (Version::from_str(&ver).unwrap(), val))
    .collect();

  fs::create_dir_all(&dir)?;
  {
    let (to_client, to_server) = to_versioned(&versions);
    let to_client = generate_packets(to_client, &versions, true)?;
    let to_server = generate_packets(to_server, &versions, false)?;

    fs::write(dir.join("cb.rs"), to_client)?;
    fs::write(dir.join("sb.rs"), to_server)?;

    // Ok(quote! {
    //   pub mod cb {
    //     #to_client
    //   }
    //   pub mod sb {
    //     #to_server
    //   }
    // })
  }
  Ok(())
}

fn generate_packets(
  packets: Vec<VersionedPacket>,
  versions: &HashMap<Version, PacketVersion>,
  to_client: bool,
) -> Result<String, Box<dyn Error>> {
  let mut gen = CodeGen::new();
  gen.write_line("use sc_transfer::{MessageRead, MessageWrite, ReadError, WriteError};");
  gen.write_line("use crate::{");
  gen.write_line("  net::tcp,");
  gen.write_line("  Pos,");
  gen.write_line("  proto,");
  gen.write_line("  version::ProtocolVersion,");
  gen.write_line("  util::{Item, UUID}");
  gen.write_line("};");
  gen.write_line("");
  gen.write_line("/// Auto generated packet ids. This is a combination of all packet");
  gen.write_line("/// names for all versions. Some of these packets are never used.");
  gen.write_line("#[derive(Clone, Debug, PartialEq)]");
  gen.write_enum(
    "Packet",
    packets
      .iter()
      .map(|packet| {
        EnumVariant::Struct(
          packet.name().to_case(Case::Pascal),
          packet.fields().into_iter().map(|field| (field.name(), field.ty())).collect(),
        )
      })
      .append_start([EnumVariant::Named("None".into())]),
  );
  gen.write_impl("Packet", |gen| {
    gen.write_func("id", &[FuncArg::slf_ref()], Some("i32"), |gen| {
      gen.write_match("self", |gen| {
        gen.write_match_branch(Some("Self"), MatchBranch::Unit("None"));
        gen.write_line("unreachable!(\"cannot get id of None packet\"),");
        for (id, p) in packets.iter().enumerate() {
          gen.write_match_branch(
            Some("Self"),
            MatchBranch::Struct(&p.name().to_case(Case::Pascal), vec![]),
          );
          gen.write_line(&format!("{},", id));
        }
      });
    });
    gen.write_func(
      "to_proto",
      &[
        FuncArg::slf_ref(),
        FuncArg { name: "version", ty: "ProtocolVersion" },
        FuncArg { name: "gargage", ty: "&mut [u8]" },
      ],
      Some("Result<usize, WriteError>"),
      |gen| {
        gen.write_match("self", |gen| {
          for (id, p) in packets.iter().enumerate() {
            gen.write_match_branch(
              Some("Self"),
              MatchBranch::Struct(
                &p.name().to_case(Case::Pascal),
                p.fields().into_iter().map(|field| field.name()).collect(),
              ),
            );
            gen.write_block(|gen| {
              gen.write_line("let mut m = MessageWrite::new(&mut garbage);");
              gen.write("m.write_varint(");
              gen.write(&id.to_string());
              gen.write_line(")?; // sc id");
              if p.has_multiple_versions() {
                let all_versions = p.all_versions();
                for (i, ver) in all_versions.iter().enumerate() {
                  if i == 0 && *ver != (Version { major: 8, minor: 0 }) {
                    gen.write("if version < ProtocolVersion::");
                    gen.write(&ver.to_string().to_uppercase());
                    gen.write_line(" {");
                    gen.add_indent();
                    gen.write_comment("1.8 generator");
                    for (is_ver, field) in p.fields_ver(Version { major: 8, minor: 0 }).iter() {
                      if *is_ver {
                        field.write_to_proto(gen);
                      }
                    }
                    gen.remove_indent();
                    gen.write("} else ");
                  } else if i != 0 {
                    gen.write(" else ");
                  }
                  if let Some(next_ver) = all_versions.get(i + 1) {
                    gen.write("if version < ProtocolVersion::");
                    gen.write(&next_ver.to_string().to_uppercase());
                    gen.write(" ");
                  }
                  gen.write_line("{");
                  gen.add_indent();
                  gen.write_comment(&ver.to_string());
                  for (is_ver, field) in p.fields_ver(*ver).iter() {
                    if *is_ver {
                      field.write_to_proto(gen);
                    }
                  }
                  gen.remove_indent();
                  gen.write("}");
                }
                gen.write_line("");
              } else {
                for field in p.fields().iter() {
                  field.write_to_proto(gen);
                }
              }
              gen.write_line("Ok(m.index())");
            });
          }
          gen.write_match_branch(Some("Self"), MatchBranch::Unit("None"));
          gen.write_line("unreachable!(\"cannot convert None packet to proto\"),");
        });
      },
    );
    gen.write_func(
      "from_proto",
      &[
        FuncArg { name: "mut pb", ty: "proto::Packet" },
        FuncArg { name: "version", ty: "ProtocolVersion" },
      ],
      Some("Self"),
      |gen| {
        gen.write_match("pb.id", |gen| {
          for (id, p) in packets.iter().enumerate() {
            gen.write_match_branch(None, MatchBranch::Unit(&id.to_string()));
            if p.has_multiple_versions() {
              let all_versions = p.all_versions();
              // We are in a struct literal now
              for (i, ver) in all_versions.iter().enumerate() {
                if i == 0 && *ver != (Version { major: 8, minor: 0 }) {
                  gen.write("if version < ProtocolVersion::");
                  gen.write(&ver.to_string().to_uppercase());
                  gen.write_line(" {");
                  gen.add_indent();
                  gen.write_comment("1.8 generator");
                  gen.write("Packet::");
                  gen.write(&p.name().to_case(Case::Pascal));
                  gen.write(" ");
                  gen.write_block(|gen| {
                    for (is_ver, field) in p.fields_ver(Version { major: 8, minor: 0 }).iter().rev()
                    {
                      field.write_from_proto(gen, *is_ver);
                    }
                  });
                  gen.remove_indent();
                  gen.write("} else ");
                } else if i != 0 {
                  gen.write(" else ");
                }
                if let Some(next_ver) = all_versions.get(i + 1) {
                  gen.write("if version < ProtocolVersion::");
                  gen.write(&next_ver.to_string().to_uppercase());
                  gen.write(" ");
                }
                gen.write_line("{");
                gen.add_indent();
                gen.write_comment(&ver.to_string());
                gen.write("Packet::");
                gen.write(&p.name().to_case(Case::Pascal));
                gen.write(" ");
                gen.write_block(|gen| {
                  for (is_ver, field) in p.fields_ver(*ver).iter().rev() {
                    field.write_from_proto(gen, *is_ver);
                  }
                });
                gen.remove_indent();
                gen.write("}");
              }
              gen.write_line(",");
            } else {
              gen.write("Packet::");
              gen.write(&p.name().to_case(Case::Pascal));
              gen.write_line(" {");
              gen.add_indent();
              for field in p.fields().iter().rev() {
                field.write_from_proto(gen, true);
              }
              gen.remove_indent();
              gen.write_line("},");
            }
          }
          gen.write_match_branch(None, MatchBranch::Other);
          gen.write_line("unreachable!(\"invalid packet id {}\", pb.id),");
        });
      },
    );
    gen.write_func(
      "to_tcp",
      &[FuncArg::slf_ref(), FuncArg { name: "version", ty: "ProtocolVersion" }],
      Some("tcp::Packet"),
      |gen| {
        gen.write_match("self", |gen| {
          gen.write_match_branch(Some("Self"), MatchBranch::Unit("None"));
          gen.write_line("unreachable!(\"cannot convert None packet to tcp\"),");
          for (id, p) in packets.iter().enumerate() {
            gen.write_match_branch(
              Some("Self"),
              MatchBranch::Struct(
                &p.name().to_case(Case::Pascal),
                p.fields().into_iter().map(|field| field.name()).collect(),
              ),
            );
            gen.write_block(|gen| {
              gen.write("let mut out = tcp::Packet::new(from_grpc_id(");
              gen.write(&id.to_string());
              gen.write_line(", version), version);");
              if p.has_multiple_versions() {
                let all_versions = p.all_versions();
                for (i, ver) in all_versions.iter().enumerate() {
                  if i == 0 && *ver != (Version { major: 8, minor: 0 }) {
                    gen.write("if version < ProtocolVersion::");
                    gen.write(&ver.to_string().to_uppercase());
                    gen.write_line(" {");
                    gen.add_indent();
                    gen.write_comment("1.8 generator");
                    for (is_ver, field) in p.fields_ver(Version { major: 8, minor: 0 }).iter() {
                      if *is_ver {
                        gen.write(&field.generate_to_tcp().to_string());
                        gen.write_line(";");
                      }
                    }
                    gen.remove_indent();
                    gen.write("} else ");
                  } else if i != 0 {
                    gen.write(" else ");
                  }
                  if let Some(next_ver) = all_versions.get(i + 1) {
                    gen.write("if version < ProtocolVersion::");
                    gen.write(&next_ver.to_string().to_uppercase());
                    gen.write(" ");
                  }
                  gen.write_line("{");
                  gen.add_indent();
                  gen.write_comment(&ver.to_string());
                  for (is_ver, field) in p.fields_ver(*ver).iter() {
                    if *is_ver {
                      gen.write(&field.generate_to_tcp().to_string());
                      gen.write_line(";");
                    }
                  }
                  gen.remove_indent();
                  gen.write("}");
                }
                gen.write_line("");
              } else {
                for field in p.fields().iter() {
                  gen.write(&field.generate_to_tcp());
                  gen.write_line(";");
                }
              }
              gen.write_line("out");
            });
          }
        });
      },
    );
    gen.write_func(
      "from_tcp",
      &[
        FuncArg { name: "mut p", ty: "tcp::Packet" },
        FuncArg { name: "version", ty: "ProtocolVersion" },
      ],
      Some("Self"),
      |gen| {
        gen.write_match("to_grpc_id(p.id(), version)", |gen| {
          for (id, p) in packets.iter().enumerate() {
            gen.write_match_branch(None, MatchBranch::Unit(&id.to_string()));
            if p.has_multiple_versions() {
              let all_versions = p.all_versions();
              for (i, ver) in all_versions.iter().enumerate() {
                if i == 0 && *ver != (Version { major: 8, minor: 0 }) {
                  gen.write("if version < ProtocolVersion::");
                  gen.write(&ver.to_string().to_uppercase());
                  gen.write_line(" {");
                  gen.add_indent();
                  gen.write_comment("1.8 generator");
                  gen.write("Packet::");
                  gen.write(&p.name().to_case(Case::Pascal));
                  gen.write(" ");
                  gen.write_block(|gen| {
                    for (is_ver, field) in p.fields_ver(Version { major: 8, minor: 0 }).iter() {
                      field.write_from_tcp(gen, *is_ver);
                    }
                  });
                  gen.remove_indent();
                  gen.write("} else ");
                } else if i != 0 {
                  gen.write(" else ");
                }
                if let Some(next_ver) = all_versions.get(i + 1) {
                  gen.write("if version < ProtocolVersion::");
                  gen.write(&next_ver.to_string().to_uppercase());
                  gen.write(" ");
                }
                gen.write_line("{");
                gen.add_indent();
                gen.write_comment(&ver.to_string());
                gen.write("Packet::");
                gen.write(&p.name().to_case(Case::Pascal));
                gen.write(" ");
                gen.write_block(|gen| {
                  for (is_ver, field) in p.fields_ver(*ver).iter() {
                    field.write_from_tcp(gen, *is_ver);
                  }
                });
                gen.remove_indent();
                gen.write("}");
              }
              gen.write_line(",");
            } else {
              gen.write("Packet::");
              gen.write(&p.name().to_case(Case::Pascal));
              gen.write_line(" {");
              gen.add_indent();
              for field in p.fields().iter() {
                field.write_from_tcp(gen, true);
              }
              gen.remove_indent();
              gen.write_line("},");
            }
          }
          gen.write_match_branch(None, MatchBranch::Other);
          gen.write_line("unreachable!(\"invalid packet id {}\", p.id()),");
        });
      },
    );
  });

  gen.write_line("/// Converts a grpc packet id into a tcp packet id");
  gen.write_func(
    "from_grpc_id",
    &[FuncArg { name: "id", ty: "i32" }, FuncArg { name: "ver", ty: "ProtocolVersion" }],
    Some("i32"),
    |gen| {
      gen.write_match("ver", |gen| {
        for (ver_name, ver) in versions.iter().sorted_by(|(ver_a, _), (ver_b, _)| ver_a.cmp(ver_b))
        {
          gen.write_match_branch(
            Some("ProtocolVersion"),
            MatchBranch::Unit(&ver_name.to_string().to_uppercase()),
          );
          gen.write_match("id", |gen| {
            let tcp_packets = if to_client { &ver.to_client } else { &ver.to_server };
            for (tcp_id, tcp_packet) in tcp_packets.iter().enumerate() {
              let grpc_id = packets
                .binary_search_by(|grpc_packet| grpc_packet.name.cmp(&tcp_packet.name))
                .unwrap();
              gen.write_match_branch(None, MatchBranch::Unit(&grpc_id.to_string()));
              gen.write(&tcp_id.to_string());
              gen.write_line(",");
            }
            gen.write_line("_ => panic!(\"unknown grpc id {}\", id),");
          });
          gen.write_line(",");
        }
        gen.write_line("ver => panic!(\"invalid version {:?}\", ver),");
      });
    },
  );
  gen.write_line("/// Converts a grpc packet id into a tcp packet id");
  gen.write_func(
    "to_grpc_id",
    &[FuncArg { name: "id", ty: "i32" }, FuncArg { name: "ver", ty: "ProtocolVersion" }],
    Some("i32"),
    |gen| {
      gen.write_match("ver", |gen| {
        for (ver_name, ver) in versions.iter().sorted_by(|(ver_a, _), (ver_b, _)| ver_a.cmp(ver_b))
        {
          gen.write_match_branch(
            Some("ProtocolVersion"),
            MatchBranch::Unit(&ver_name.to_string().to_uppercase()),
          );
          gen.write_match("id", |gen| {
            let tcp_packets = if to_client { &ver.to_client } else { &ver.to_server };
            for (tcp_id, tcp_packet) in tcp_packets.iter().enumerate() {
              let grpc_id = packets
                .binary_search_by(|grpc_packet| grpc_packet.name.cmp(&tcp_packet.name))
                .unwrap();
              gen.write_match_branch(None, MatchBranch::Unit(&tcp_id.to_string()));
              gen.write(&grpc_id.to_string());
              gen.write_line(",");
            }
            gen.write_line("_ => panic!(\"unknown tcp id {}\", id),");
          });
          gen.write_line(",");
        }
        gen.write_line("ver => panic!(\"invalid version {:?}\", ver),");
      });
    },
  );

  Ok(gen.into_output())
}
