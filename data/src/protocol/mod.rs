mod json;
mod parse;

use crate::gen::{AppendIters, CodeGen, EnumVariant, FuncArg, MatchBranch};
use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use serde_derive::{Deserialize, Serialize};
use std::{
  cmp,
  collections::{HashMap, HashSet},
  error::Error,
  fmt, fs,
  path::Path,
  str::FromStr,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum IntType {
  I8,
  U8,
  U16,
  I16,
  I32,
  I64,
  VarInt,
  OptVarInt, // Acts the same as a varint, but is sometimes not present
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum FloatType {
  F32,
  F64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum CountType {
  // A typed count
  Typed(IntType),
  // A hardocded count
  Fixed(u32),
  // Another protocol field should be used as the count
  Named(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BitField {
  name:   String,
  size:   u32,
  signed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum PacketField {
  // Simple fields
  Native, // Should never exist
  Bool,
  Int(IntType),
  Float(FloatType),
  UUID,
  String,
  Position,

  // Sizable fields
  NBT,
  OptionalNBT,
  RestBuffer, // The rest of the buffer
  EntityMetadata,

  // Complicated fields
  Option(Box<PacketField>),
  Array { count: CountType, value: Box<PacketField> },
  Buffer(CountType),
  BitField(Vec<BitField>),
  Container(Container),
  Switch { compare_to: String, fields: HashMap<String, PacketField> },
  Mappings(HashMap<String, u32>), // Mapping of packet names to ids

  // Logical fields
  CompareTo(String),
  DefinedType(String), // Another type, defined within either the types map or the packets map
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Container {
  pub fields: Vec<(String, PacketField)>,
  pub names:  HashMap<String, usize>,
}

impl Container {
  pub fn get(&self, n: &str) -> &PacketField {
    &self.fields[self.names[n]].1
  }
}

impl PacketField {
  pub fn into_container(self) -> Option<Container> {
    match self {
      Self::Container(v) => Some(v),
      _ => None,
    }
  }
  pub fn into_defined(self) -> Option<String> {
    match self {
      Self::DefinedType(v) => Some(v),
      _ => None,
    }
  }
}

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

struct NamedPacketField {
  multi_versioned: bool,
  name:            String,
  field:           PacketField,
}

struct VersionedField {
  name:          String,
  version_names: HashMap<Version, usize>,
  versions:      Vec<(Version, PacketField)>,
}

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

impl VersionedField {
  fn new(ver: Version, name: String, field: PacketField) -> Self {
    let mut version_names = HashMap::new();
    version_names.insert(ver, 0);
    VersionedField { name, version_names, versions: vec![(ver, field)] }
  }
  fn add_ver(&mut self, ver: Version, field: PacketField) {
    self.version_names.insert(ver, self.versions.len());
    self.versions.push((ver, field));
  }
  fn latest(&self) -> &PacketField {
    &self.versions.last().unwrap().1
  }
  fn add_all(&self, out: &mut Vec<NamedPacketField>) {
    if self.versions.len() == 1 {
      // If the first version is not 1.8, we need this to be a multi versioned field
      let multi_versioned = self.versions.first().unwrap().0 != Version { major: 8, minor: 0 };
      let mut name = self.name.clone();
      if multi_versioned {
        name.push_str("_");
        name.push_str(&self.versions.first().unwrap().0.to_string());
      }
      // Avoid keyword conflicts
      if name == "type" {
        name = "type_".to_string();
      }
      out.push(NamedPacketField {
        multi_versioned,
        name,
        field: self.versions.first().unwrap().1.clone(),
      });
    } else {
      for (ver, field) in &self.versions {
        let mut name = self.name.clone();
        name.push_str("_");
        name.push_str(&ver.to_string());
        out.push(NamedPacketField { multi_versioned: true, name, field: field.clone() });
      }
    }
  }
  fn multi_versioned(&self) -> bool {
    self.versions.len() > 1 || self.versions.first().unwrap().0 != Version { major: 8, minor: 0 }
  }
  fn add_all_ver(&self, out: &mut Vec<(bool, NamedPacketField)>, matching_ver: Version) {
    if self.versions.len() == 1 {
      // If the first version is not 1.8, we need this to be a multi versioned field
      let multi_versioned = self.versions.first().unwrap().0 != Version { major: 8, minor: 0 };
      let mut name = self.name.clone();
      if multi_versioned {
        name.push_str("_");
        name.push_str(&self.versions.first().unwrap().0.to_string());
      }
      // Avoid keyword conflicts
      if name == "type" {
        name = "type_".to_string();
      }
      out.push((
        // Make sure that we don't set is_ver to true for a field that hasn't been added to this
        // packet yet.
        matching_ver >= self.versions.first().unwrap().0,
        NamedPacketField { multi_versioned, name, field: self.versions.first().unwrap().1.clone() },
      ));
    } else {
      let mut found_ver = false;
      for (i, (_, field)) in self.versions.iter().enumerate() {
        let this_ver = self.versions[i].0;
        let mut is_ver = false;
        if !found_ver {
          if let Some((next_ver, _)) = self.versions.get(i + 1) {
            if matching_ver >= this_ver && matching_ver < *next_ver {
              found_ver = true;
              is_ver = true;
            }
          } else {
            if matching_ver >= this_ver {
              found_ver = true;
              is_ver = true;
            }
          }
        }
        let mut name = self.name.clone();
        name.push_str("_");
        name.push_str(&this_ver.to_string());
        out.push((is_ver, NamedPacketField { multi_versioned: true, name, field: field.clone() }));
      }
    }
  }
}

impl VersionedPacket {
  fn new(name: String) -> Self {
    VersionedPacket { name, field_names: HashMap::new(), fields: vec![] }
  }

  fn add_version(&mut self, ver: Version, packet: Packet) {
    for (name, field) in packet.fields {
      if let Some(&idx) = self.field_names.get(&name) {
        let existing = &mut self.fields[idx];
        if existing.latest() != &field {
          existing.add_ver(ver, field);
        }
      } else {
        self.field_names.insert(name.clone(), self.fields.len());
        self.fields.push(VersionedField::new(ver, name, field));
      }
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
      format!("Option<{}>", self.field.ty_lit().to_string())
    } else {
      self.field.ty_lit().to_string()
    }
  }
  fn write_to_proto(&self, gen: &mut CodeGen) {
    gen.write_line("fields.push(proto::PacketField {");
    gen.add_indent();
    gen.write(&self.field.ty_key().to_string());
    gen.write(": ");
    gen.write(&self.generate_to_proto().to_string());
    gen.write_line(",");
    gen.write_line("..Default::default()");
    gen.remove_indent();
    gen.write_line("});");
  }
  fn write_from_proto(&self, gen: &mut CodeGen, is_ver: bool) {
    gen.write(&self.name());
    gen.write(": ");
    if self.multi_versioned {
      if is_ver {
        gen.write("Some(");
        gen.write(&self.generate_from_proto().to_string());
        gen.write(")");
      } else {
        gen.write("None");
      }
    } else {
      gen.write(&self.generate_from_proto().to_string());
    }
    gen.write_line(",");
  }
  fn write_from_tcp(&self, gen: &mut CodeGen, is_ver: bool) {
    gen.write(&self.name());
    gen.write(": ");
    if self.multi_versioned {
      if is_ver {
        gen.write("Some(");
        gen.write(&self.generate_from_tcp().to_string());
        gen.write(")");
      } else {
        gen.write("None");
      }
    } else {
      gen.write(&self.generate_from_tcp().to_string());
    }
    gen.write_line(",");
  }
  fn generate_to_proto(&self) -> String {
    if self.multi_versioned {
      self.field.generate_to_proto(&format!("{}.as_ref().unwrap()", self.name))
    } else {
      self.field.generate_to_proto(&self.name)
    }
  }
  fn generate_from_proto(&self) -> String {
    self.field.generate_from_proto().to_string()
  }
  fn generate_to_tcp(&self) -> String {
    if self.multi_versioned {
      self.field.generate_to_tcp(&format!("{}.as_ref().unwrap()", self.name))
    } else {
      self.field.generate_to_tcp(&self.name)
    }
  }
  fn generate_from_tcp(&self) -> String {
    self.field.generate_from_tcp().to_string()
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

impl PacketField {
  fn ty_lit(&self) -> TokenStream {
    // // Simple fields
    // Native, // Should never exist
    // Bool,
    // Int(IntType),
    // Float(FloatType),
    // UUID,
    // String,
    // Position,

    // // Sizable fields
    // NBT,
    // OptionalNBT,
    // RestBuffer, // The rest of the buffer
    // EntityMetadata,

    // // Complicated fields
    // Option(Box<PacketField>),
    // Array { count: CountType, value: Box<PacketField> },
    // Buffer(CountType),
    // BitField(Vec<BitField>),
    // Container(Container),
    // Switch { compare_to: String, fields: HashMap<String, PacketField> },
    // Mappings(HashMap<String, u32>), // Mapping of packet names to ids

    // // Logical fields
    // CompareTo(String),
    // DefinedType(String), // Another type, defined within either the types map or
    // the packets map
    match self {
      Self::Bool => quote!(bool),
      Self::Int(ity) => match ity {
        IntType::I8 => quote!(i8),
        IntType::U8 => quote!(u8),
        IntType::I16 => quote!(i16),
        IntType::U16 => quote!(u16),
        IntType::I32 => quote!(i32),
        IntType::I64 => quote!(i64),
        IntType::VarInt => quote!(i32),
        IntType::OptVarInt => quote!(i32), // TODO: Might want to change this to Option<i32>
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => quote!(f32),
        FloatType::F64 => quote!(f64),
      },
      Self::UUID => quote!(UUID),
      Self::String => quote!(String),
      Self::Position => quote!(Pos),

      // Self::NBT => quote!(NBT),
      // Self::OptionalNBT => quote!(Option<NBT>),
      // Self::RestBuffer => quote!(Vec<u8>),
      // Self::EntityMetadata => quote!(Vec<u8>), // Implemented on the server

      // Self::Option(field) => {
      //   let inner = field.ty_lit();
      //   quote!(Option<#inner>)
      // }
      // Self::Array { count, value } => match count {
      //   CountType::Typed(_) | CountType::Named(_) => {
      //     let value = value.ty_lit();
      //     quote!(Vec<#value>)
      //   }
      //   CountType::Fixed(len) => {
      //     let value = value.ty_lit();
      //     quote!([#value; #len])
      //   }
      // },
      _ => quote!(Vec<u8>),
    }
  }
  fn ty_key(&self) -> TokenStream {
    // The int types are:
    // `sint` -> Signed, variable length encoded
    // `uint` -> Unsigned, variable length encoded
    // `int` -> Signed, fixed length encoded
    //
    // So:
    // `sint` -> i8, i16, varint
    // `uint` -> Any unsigned int
    // `int` -> i32
    match self {
      Self::Bool => quote!(bool),
      Self::Int(ity) => match ity {
        IntType::I8 => quote!(sint),
        IntType::U8 => quote!(uint),
        IntType::I16 => quote!(sint),
        IntType::U16 => quote!(uint),
        IntType::I32 => quote!(int),
        IntType::I64 => quote!(long),
        IntType::VarInt => quote!(sint),
        IntType::OptVarInt => quote!(sint),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => quote!(float),
        FloatType::F64 => quote!(double),
      },
      Self::UUID => quote!(uuid),
      Self::String => quote!(str),
      Self::Position => quote!(pos),

      // Self::NBT => quote!(byte_arr),
      // Self::OptionalNBT => quote!(byte_arr),
      // Self::RestBuffer => quote!(byte_arr),
      // Self::EntityMetadata => quote!(byte_arr), // Implemented on the server

      // Self::Option(field) => field.ty_key(),
      _ => quote!(byte_arr),
    }
  }
  fn generate_to_proto(&self, val: &str) -> String {
    match self {
      Self::Bool => format!("*{}", val),
      Self::Int(ity) => match ity {
        IntType::I8 => format!("(*{}).into()", val),
        IntType::U8 => format!("(*{}).into()", val),
        IntType::I16 => format!("(*{}).into()", val),
        IntType::U16 => format!("(*{}).into()", val),
        IntType::I32 => format!("*{}", val),
        IntType::I64 => format!("*{} as u64", val),
        IntType::VarInt => format!("*{}", val),
        IntType::OptVarInt => format!("{}.unwrap_or(0)", val),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => format!("*{}", val),
        FloatType::F64 => format!("*{}", val),
      },
      Self::UUID => format!("Some({}.as_proto())", val),
      Self::String => format!("{}.to_string()", val),
      Self::Position => format!("{}.to_u64()", val),

      // Self::NBT => format!(#name.clone()),
      // Self::OptionalNBT => format!(#name.clone()),
      // Self::RestBuffer => format!(#name.clone()),
      // Self::EntityMetadata => format!(#name.clone()), // Implemented on the server

      // Self::Option(field) => format!(#name.unwrap()),
      _ => format!("{}.clone()", val),
    }
  }
  fn generate_from_proto(&self) -> TokenStream {
    match self {
      Self::Bool => quote!(pb.fields.pop().unwrap().bool),
      Self::Int(ity) => match ity {
        IntType::I8 => quote!(pb.fields.pop().unwrap().sint as i8),
        IntType::U8 => quote!(pb.fields.pop().unwrap().uint as u8),
        IntType::I16 => quote!(pb.fields.pop().unwrap().sint as i16),
        IntType::U16 => quote!(pb.fields.pop().unwrap().uint as u16),
        IntType::I32 => quote!(pb.fields.pop().unwrap().int),
        IntType::I64 => quote!(pb.fields.pop().unwrap().long as i64),
        IntType::VarInt => quote!(pb.fields.pop().unwrap().sint),
        IntType::OptVarInt => quote!(Some(pb.fields.pop().unwrap().sint)),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => quote!(pb.fields.pop().unwrap().float),
        FloatType::F64 => quote!(pb.fields.pop().unwrap().double),
      },
      Self::UUID => quote!(UUID::from_proto(pb.fields.pop().unwrap().uuid.unwrap())),
      Self::String => quote!(pb.fields.pop().unwrap().str),
      Self::Position => quote!(Pos::from_u64(pb.fields.pop().unwrap().pos)),

      // Self::NBT => quote!(#name.clone()),
      // Self::OptionalNBT => quote!(#name.clone()),
      // Self::RestBuffer => quote!(#name.clone()),
      // Self::EntityMetadata => quote!(#name.clone()), // Implemented on the server

      // Self::Option(field) => quote!(#name.unwrap()),
      _ => quote!(pb.fields.pop().unwrap().byte_arr),
    }
  }
  fn generate_to_tcp(&self, val: &str) -> String {
    match self {
      Self::Bool => format!("out.write_bool(*{})", val),
      Self::Int(ity) => match ity {
        IntType::I8 => format!("out.write_i8(*{})", val),
        IntType::U8 => format!("out.write_u8(*{})", val),
        IntType::I16 => format!("out.write_i16(*{})", val),
        IntType::U16 => format!("out.write_u16(*{})", val),
        IntType::I32 => format!("out.write_i32(*{})", val),
        IntType::I64 => format!("out.write_i64(*{})", val),
        IntType::VarInt => format!("out.write_varint(*{})", val),
        IntType::OptVarInt => format!("out.write_varint(*{}.unwrap_or(0))", val),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => format!("out.write_f32(*{})", val),
        FloatType::F64 => format!("out.write_f64(*{})", val),
      },
      Self::UUID => format!("out.write_uuid(*{})", val),
      Self::String => format!("out.write_str({})", val),
      Self::Position => format!("out.write_pos(*{})", val),

      // Self::NBT => quote!(#val.clone()),
      // Self::OptionalNBT => quote!(#val.clone()),
      // Self::RestBuffer => quote!(#val.clone()),
      // Self::EntityMetadata => quote!(#val.clone()), // Implemented on the server

      // Self::Option(field) => quote!(#val.unwrap()),
      _ => format!("out.write_buf({})", val),
    }
  }
  fn generate_from_tcp(&self) -> TokenStream {
    match self {
      Self::Bool => quote!(p.read_bool()),
      Self::Int(ity) => match ity {
        IntType::I8 => quote!(p.read_i8()),
        IntType::U8 => quote!(p.read_u8()),
        IntType::I16 => quote!(p.read_i16()),
        IntType::U16 => quote!(p.read_u16()),
        IntType::I32 => quote!(p.read_i32()),
        IntType::I64 => quote!(p.read_i64()),
        IntType::VarInt => quote!(p.read_varint()),
        IntType::OptVarInt => quote!(Some(p.read_varint())),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => quote!(p.read_f32()),
        FloatType::F64 => quote!(p.read_f64()),
      },
      Self::UUID => quote!(p.read_uuid()),
      Self::String => quote!(p.read_str()),
      Self::Position => quote!(p.read_pos()),

      // Self::NBT => quote!(#name.clone()),
      // Self::OptionalNBT => quote!(#name.clone()),
      // Self::RestBuffer => quote!(#name.clone()),
      // Self::EntityMetadata => quote!(#name.clone()), // Implemented on the server

      // Self::Option(field) => quote!(#name.unwrap()),
      _ => quote!({
        let len = p.read_varint();
        p.read_buf(len)
      }),
    }
  }
}

fn generate_packets(
  packets: Vec<VersionedPacket>,
  versions: &HashMap<Version, PacketVersion>,
  to_client: bool,
) -> Result<String, Box<dyn Error>> {
  let mut gen = CodeGen::new();
  gen.write_line("use crate::{");
  gen.write_line("  math::Pos,");
  gen.write_line("  net::tcp,");
  gen.write_line("  proto,");
  gen.write_line("  version::ProtocolVersion,");
  gen.write_line("  util::UUID");
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
      &[FuncArg::slf_ref(), FuncArg { name: "version", ty: "ProtocolVersion" }],
      Some("proto::Packet"),
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
              // TODO: Use with_capacity here
              gen.write_line("let mut fields = Vec::new();");
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
              gen.write_line("proto::Packet{");
              gen.add_indent();
              gen.write("id: ");
              gen.write(&id.to_string());
              gen.write_line(",");
              gen.write_line("fields,");
              gen.remove_indent();
              gen.write_line("}");
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
                    for (i, (is_ver, field)) in
                      p.fields_ver(Version { major: 8, minor: 0 }).iter().enumerate()
                    {
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
                  for (i, (is_ver, field)) in p.fields_ver(*ver).iter().enumerate() {
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
                for (i, field) in p.fields().iter().enumerate() {
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
                    for (i, (is_ver, field)) in
                      p.fields_ver(Version { major: 8, minor: 0 }).iter().enumerate()
                    {
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
                  for (i, (is_ver, field)) in p.fields_ver(*ver).iter().enumerate() {
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
              for (i, field) in p.fields().iter().enumerate() {
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
