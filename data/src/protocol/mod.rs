mod json;
mod parse;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
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
  name:  String,
  field: PacketField,
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
  fn add_all(&self, out: &mut Vec<(bool, NamedPacketField)>) {
    if self.versions.len() == 1 {
      let mut name = self.name.clone();
      // Avoid keyword conflicts
      if name == "type" {
        name = "type_".to_string();
      }
      out.push((false, NamedPacketField { name, field: self.versions.first().unwrap().1.clone() }));
    } else {
      for (ver, field) in &self.versions {
        let mut name = self.name.clone();
        name.push_str("_");
        name.push_str(&ver.to_string());
        out.push((true, NamedPacketField { name, field: field.clone() }));
      }
    }
  }
  fn add_all_ver(&self, out: &mut Vec<(bool, bool, NamedPacketField)>, matching_ver: Version) {
    if self.versions.len() == 1 {
      let mut name = self.name.clone();
      // Avoid keyword conflicts
      if name == "type" {
        name = "type_".to_string();
      }
      out.push((
        true,
        false,
        NamedPacketField { name, field: self.versions.first().unwrap().1.clone() },
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
            found_ver = true;
            is_ver = true;
          }
        }
        let mut name = self.name.clone();
        name.push_str("_");
        name.push_str(&this_ver.to_string());
        out.push((is_ver, true, NamedPacketField { name, field: field.clone() }));
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

  fn fields(&self) -> Vec<(bool, NamedPacketField)> {
    let mut out = vec![];
    for field in &self.fields {
      field.add_all(&mut out);
    }
    out
  }

  fn fields_ver(&self, ver: Version) -> Vec<(bool, bool, NamedPacketField)> {
    let mut out = vec![];
    for field in &self.fields {
      field.add_all_ver(&mut out, ver);
    }
    out
  }

  fn has_multiple_versions(&self) -> bool {
    for field in &self.fields {
      if field.versions.len() > 1 {
        return true;
      }
    }
    false
  }
  fn all_versions(&self) -> Vec<Version> {
    let mut versions = HashSet::new();
    for field in &self.fields {
      if field.versions.len() > 1 {
        for (ver, _) in &field.versions {
          versions.insert(ver.clone());
        }
      }
    }
    versions.into_iter().sorted().collect()
  }
  fn all_field_names_ver(&self, ver: Version) -> Vec<(bool, bool, String, String)> {
    let mut vals = vec![];
    for (for_ver, multi_versioned, field) in self.fields_ver(ver) {
      vals.push((for_ver, multi_versioned, field.name.clone(), field.field.ty_lit().to_string()));
    }
    vals
  }
  fn field_name_tys(&self) -> Vec<(String, String)> {
    let mut vals = vec![];
    for (multi_versioned, field) in self.fields() {
      vals.push((
        field.name.clone(),
        if multi_versioned {
          format!("Option<{}>", field.field.ty_lit().to_string())
        } else {
          field.field.ty_lit().to_string()
        },
      ));
    }
    vals
  }
  fn field_ty_enums(&self) -> Vec<TokenStream> {
    let mut tys = vec![];
    for (_multi_versioned, field) in self.fields() {
      tys.push(field.field.ty_enum());
    }
    tys
  }
  fn field_ty_keys(&self) -> Vec<TokenStream> {
    let mut tys = vec![];
    for (_multi_versioned, field) in self.fields() {
      tys.push(field.field.ty_key());
    }
    tys
  }
  fn field_to_protos(&self) -> Vec<String> {
    let mut vals = vec![];
    for (multi_versioned, field) in self.fields() {
      if multi_versioned {
        vals.push(field.field.generate_to_proto(&format!("{}.as_ref().unwrap()", field.name)));
      } else {
        vals.push(field.field.generate_to_proto(&field.name));
      }
    }
    vals
  }
  fn field_from_protos(&self) -> Vec<TokenStream> {
    let mut vals = vec![];
    for (_multi_versioned, field) in self.fields() {
      vals.push(field.field.generate_from_proto(&field.name));
    }
    vals
  }
  fn field_to_tcps(&self) -> Vec<String> {
    let mut vals = vec![];
    for (multi_versioned, field) in self.fields() {
      if multi_versioned {
        vals.push(field.field.generate_to_tcp(&format!("{}.as_ref().unwrap()", field.name)));
      } else {
        vals.push(field.field.generate_to_tcp(&field.name));
      }
    }
    vals
  }
  fn field_from_tcps(&self) -> Vec<TokenStream> {
    let mut vals = vec![];
    for (_multi_versioned, field) in self.fields() {
      vals.push(field.field.generate_from_tcp());
    }
    vals
  }
  fn name(&self) -> &str {
    &self.name
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
  fn ty_enum(&self) -> TokenStream {
    // enum Type {
    //   Bool    = 0;
    //   Byte    = 1;
    //   Short   = 2;
    //   Int     = 3;
    //   Long    = 4;
    //   Float   = 5;
    //   Double  = 6;
    //   Str     = 7;
    //   UUID    = 8;
    //   Pos     = 9;
    //   ByteArr = 10;
    //   IntArr  = 11;
    //   LongArr = 12;
    //   StrArr  = 13;
    // }
    match self {
      Self::Bool => quote!(Bool),
      Self::Int(ity) => match ity {
        IntType::I8 => quote!(Byte),
        IntType::U8 => quote!(Byte),
        IntType::I16 => quote!(Short),
        IntType::U16 => quote!(Short),
        IntType::I32 => quote!(Int),
        IntType::I64 => quote!(Long),
        IntType::VarInt => quote!(Int),
        IntType::OptVarInt => quote!(Int),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => quote!(Float),
        FloatType::F64 => quote!(Double),
      },
      Self::UUID => quote!(Uuid),
      Self::String => quote!(Str),
      Self::Position => quote!(Pos),

      // Self::NBT => quote!(ByteArr),
      // Self::OptionalNBT => quote!(ByteArr),
      // Self::RestBuffer => quote!(ByteArr),
      // Self::EntityMetadata => quote!(ByteArr), // Implemented on the server

      // Self::Option(field) => field.ty_lit(),
      _ => quote!(ByteArr),
    }
  }
  fn ty_key(&self) -> TokenStream {
    // enum Type {
    //   Bool    = 0;
    //   Byte    = 1;
    //   Short   = 2;
    //   Int     = 3;
    //   Long    = 4;
    //   Float   = 5;
    //   Double  = 6;
    //   Str     = 7;
    //   UUID    = 8;
    //   Pos     = 9;
    //   ByteArr = 10;
    //   IntArr  = 11;
    //   LongArr = 12;
    //   StrArr  = 13;
    // }
    match self {
      Self::Bool => quote!(bool),
      Self::Int(ity) => match ity {
        IntType::I8 => quote!(byte),
        IntType::U8 => quote!(byte),
        IntType::I16 => quote!(short),
        IntType::U16 => quote!(short),
        IntType::I32 => quote!(int),
        IntType::I64 => quote!(long),
        IntType::VarInt => quote!(int),
        IntType::OptVarInt => quote!(int),
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
        IntType::I8 => format!("(*{} as u8).into()", val),
        IntType::U8 => format!("(*{}).into()", val),
        IntType::I16 => format!("(*{} as u16).into()", val),
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
  fn generate_from_proto(&self, name: &str) -> TokenStream {
    // let name = Ident::new(name, Span::call_site());
    match self {
      Self::Bool => quote!(pb.fields[#name].bool),
      Self::Int(ity) => match ity {
        IntType::I8 => quote!(pb.fields[#name].byte as i8),
        IntType::U8 => quote!(pb.fields[#name].byte as u8),
        IntType::I16 => quote!(pb.fields[#name].short as i16),
        IntType::U16 => quote!(pb.fields[#name].short as u16),
        IntType::I32 => quote!(pb.fields[#name].int),
        IntType::I64 => quote!(pb.fields[#name].long as i64),
        IntType::VarInt => quote!(pb.fields[#name].int),
        IntType::OptVarInt => quote!(Some(pb.fields[#name].int)),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => quote!(pb.fields[#name].float),
        FloatType::F64 => quote!(pb.fields[#name].double),
      },
      Self::UUID => quote!(UUID::from_proto(pb.fields.remove(#name).unwrap().uuid.unwrap())),
      Self::String => quote!(pb.fields.remove(#name).unwrap().str),
      Self::Position => quote!(Pos::from_u64(pb.fields[#name].pos)),

      // Self::NBT => quote!(#name.clone()),
      // Self::OptionalNBT => quote!(#name.clone()),
      // Self::RestBuffer => quote!(#name.clone()),
      // Self::EntityMetadata => quote!(#name.clone()), // Implemented on the server

      // Self::Option(field) => quote!(#name.unwrap()),
      _ => quote!(pb.fields.remove(#name).unwrap().byte_arr),
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
  let mut kinds = vec![];
  let mut id_opts = vec![];
  let mut to_proto_opts = vec![];
  let mut from_proto_opts = vec![];
  let mut to_tcp_opts = vec![];
  let mut from_tcp_opts = vec![];
  for (id, packet) in packets.iter().enumerate() {
    let id = id as i32;
    let name = packet.name().to_case(Case::Pascal);
    let field_names = packet.field_name_tys();
    let field_ty_enums = packet.field_ty_enums();
    let field_ty_keys = packet.field_ty_keys();
    let field_to_protos = packet.field_to_protos();
    let field_from_protos = packet.field_from_protos();
    let field_to_tcps = packet.field_to_tcps();
    let field_from_tcps = packet.field_from_tcps();
    let mut kind = String::new();
    kind.push_str(&name);
    kind.push_str(" {\n");
    for (name, ty) in &field_names {
      kind.push_str("    ");
      kind.push_str(&name);
      kind.push_str(": ");
      kind.push_str(&ty);
      kind.push_str(",\n");
    }
    kind.push_str("  },\n");
    kinds.push(kind);
    let mut id_opt = String::new();
    id_opt.push_str("Self::");
    id_opt.push_str(&name);
    id_opt.push_str(" { .. } => ");
    id_opt.push_str(&format!("{}", id));
    id_opt.push_str(",\n");
    id_opts.push(id_opt);
    let mut proto_opt = String::new();
    proto_opt.push_str("Self::");
    proto_opt.push_str(&name);
    proto_opt.push_str(" { ");
    for (i, (name, _)) in field_names.iter().enumerate() {
      proto_opt.push_str(&name);
      if i != field_names.len() - 1 {
        proto_opt.push_str(", ");
      }
    }
    proto_opt.push_str(" } => {\n");
    proto_opt.push_str("        let mut fields = HashMap::new();\n");
    if packet.has_multiple_versions() {
      proto_opt.push_str("        ");
      for ver in packet.all_versions() {
        proto_opt.push_str("if version >= ProtocolVersion::");
        proto_opt.push_str(&ver.to_string().to_uppercase());
        proto_opt.push_str(" {\n");
        for (i, (is_ver, _multi_versioned, field_name, _)) in
          packet.all_field_names_ver(ver).iter().enumerate()
        {
          if *is_ver {
            let ty_enum = &field_ty_enums[i];
            let ty_key = &field_ty_keys[i];
            let to_proto = &field_to_protos[i];
            proto_opt.push_str("          fields.insert(\"");
            proto_opt.push_str(field_name);
            proto_opt.push_str("\".to_string(), proto::PacketField {\n");

            proto_opt.push_str("            ty: Type::");
            proto_opt.push_str(&ty_enum.to_string());
            proto_opt.push_str(".into(),\n");

            proto_opt.push_str("            ");
            proto_opt.push_str(&ty_key.to_string());
            proto_opt.push_str(": ");
            proto_opt.push_str(&to_proto.to_string());
            proto_opt.push_str(",\n");

            proto_opt.push_str("            ..Default::default()\n");
            proto_opt.push_str("          });\n");

            // tcp_opt.push_str("          ");
            // let to_tcp = &field_to_tcps[i];
            // tcp_opt.push_str(&to_tcp.to_string());
            // tcp_opt.push_str(";\n");
          }
        }
        proto_opt.push_str("        } else ");
      }
      proto_opt.push_str("{\n");
      proto_opt.push_str("          unreachable!(\"failed to generate proto for packet ");
      proto_opt.push_str(&packet.name);
      proto_opt.push_str(" with version {:?}\", version)\n");
      proto_opt.push_str("        }\n");
    } else {
      for (i, (field_name, _)) in field_names.iter().enumerate() {
        let ty_enum = &field_ty_enums[i];
        let ty_key = &field_ty_keys[i];
        let to_proto = &field_to_protos[i];
        proto_opt.push_str("        fields.insert(\"");
        proto_opt.push_str(field_name);
        proto_opt.push_str("\".to_string(), proto::PacketField {\n");

        proto_opt.push_str("          ty: Type::");
        proto_opt.push_str(&ty_enum.to_string());
        proto_opt.push_str(".into(),\n");

        proto_opt.push_str("          ");
        proto_opt.push_str(&ty_key.to_string());
        proto_opt.push_str(": ");
        proto_opt.push_str(&to_proto.to_string());
        proto_opt.push_str(",\n");

        proto_opt.push_str("          ..Default::default()\n");
        proto_opt.push_str("        });\n");
      }
    }
    proto_opt.push_str("        proto::Packet {\n");

    proto_opt.push_str("          id: ");
    proto_opt.push_str(&format!("{}", id));
    proto_opt.push_str(",\n");

    proto_opt.push_str("          fields,\n");
    proto_opt.push_str("          other: None,\n");
    proto_opt.push_str("        }\n");
    proto_opt.push_str("      }\n");
    to_proto_opts.push(proto_opt);

    let mut proto_opt = String::new();
    proto_opt.push_str(id.to_string().as_str());
    proto_opt.push_str(" => ");
    if packet.has_multiple_versions() {
      for ver in packet.all_versions() {
        proto_opt.push_str("if version >= ProtocolVersion::");
        proto_opt.push_str(&ver.to_string().to_uppercase());
        proto_opt.push_str(" {\n");
        proto_opt.push_str("        Self::");
        proto_opt.push_str(&name);
        proto_opt.push_str(" {\n");
        for (i, (is_ver, multi_versioned, field_name, _)) in
          packet.all_field_names_ver(ver).iter().enumerate()
        {
          proto_opt.push_str("          ");
          proto_opt.push_str(field_name);
          proto_opt.push_str(": ");
          if *is_ver {
            let from_proto = &field_from_protos[i];
            if *multi_versioned {
              proto_opt.push_str("Some(");
              proto_opt.push_str(&from_proto.to_string());
              proto_opt.push_str(")");
            } else {
              proto_opt.push_str(&from_proto.to_string());
            }
          } else {
            proto_opt.push_str("None");
          }
          proto_opt.push_str(",\n");
        }
        proto_opt.push_str("        }\n");
        proto_opt.push_str("      } else ");
      }
      proto_opt.push_str("{\n");
      proto_opt.push_str("        unreachable!(\"failed to parse proto for packet ");
      proto_opt.push_str(&packet.name);
      proto_opt.push_str(" with version {:?}\", version)\n");
    } else {
      proto_opt.push_str("Self::");
      proto_opt.push_str(&name);
      proto_opt.push_str(" {\n");
      for (i, (field_name, _)) in field_names.iter().enumerate() {
        let from_proto = &field_from_protos[i];
        proto_opt.push_str("        ");
        proto_opt.push_str(field_name);
        proto_opt.push_str(": ");
        proto_opt.push_str(&from_proto.to_string());
        proto_opt.push_str(",\n");
      }
    }
    proto_opt.push_str("      },\n");
    from_proto_opts.push(proto_opt);

    let mut tcp_opt = String::new();
    tcp_opt.push_str("Self::");
    tcp_opt.push_str(&name);
    tcp_opt.push_str(" { ");
    for (i, (name, _)) in field_names.iter().enumerate() {
      tcp_opt.push_str(&name);
      if i != field_names.len() - 1 {
        tcp_opt.push_str(", ");
      }
    }
    tcp_opt.push_str(" } => {\n");
    tcp_opt.push_str("        let mut out = tcp::Packet::new(from_grpc_id(");
    tcp_opt.push_str(&id.to_string());
    tcp_opt.push_str(", version), version);\n");
    if packet.has_multiple_versions() {
      tcp_opt.push_str("        ");
      for ver in packet.all_versions() {
        tcp_opt.push_str("if version >= ProtocolVersion::");
        tcp_opt.push_str(&ver.to_string().to_uppercase());
        tcp_opt.push_str(" {\n");
        for (i, (is_ver, _multi_versioned, _, _)) in
          packet.all_field_names_ver(ver).iter().enumerate()
        {
          if *is_ver {
            tcp_opt.push_str("          ");
            let to_tcp = &field_to_tcps[i];
            tcp_opt.push_str(&to_tcp.to_string());
            tcp_opt.push_str(";\n");
          }
        }
        tcp_opt.push_str("        } else ");
      }
      tcp_opt.push_str("{\n");
      tcp_opt.push_str("          unreachable!(\"failed to generate packet ");
      tcp_opt.push_str(&packet.name);
      tcp_opt.push_str(" with version {:?}\", version)\n");
      tcp_opt.push_str("        }\n");
    } else {
      for gen in &field_to_tcps {
        tcp_opt.push_str("        ");
        tcp_opt.push_str(&gen.to_string());
        tcp_opt.push_str(";\n");
      }
    }
    tcp_opt.push_str("        out\n");
    tcp_opt.push_str("      }\n");
    to_tcp_opts.push(tcp_opt);

    let mut tcp_opt = String::new();
    if packet.has_multiple_versions() {
      tcp_opt.push_str(&id.to_string());
      tcp_opt.push_str(" => ");
      for ver in packet.all_versions() {
        tcp_opt.push_str("if version >= ProtocolVersion::");
        tcp_opt.push_str(&ver.to_string().to_uppercase());
        tcp_opt.push_str(" {\n");
        tcp_opt.push_str("        Self::");
        tcp_opt.push_str(&name);
        tcp_opt.push_str(" {\n");
        for (i, (is_ver, multi_versioned, field_name, _)) in
          packet.all_field_names_ver(ver).iter().enumerate()
        {
          tcp_opt.push_str("          ");
          tcp_opt.push_str(field_name);
          tcp_opt.push_str(": ");
          if *is_ver {
            let from_tcp = &field_from_tcps[i];
            if *multi_versioned {
              tcp_opt.push_str("Some(");
              tcp_opt.push_str(&from_tcp.to_string());
              tcp_opt.push_str(")");
            } else {
              tcp_opt.push_str(&from_tcp.to_string());
            }
          } else {
            tcp_opt.push_str("None");
          }
          tcp_opt.push_str(",\n");
        }
        tcp_opt.push_str("        }\n");
        tcp_opt.push_str("      } else ");
      }
      tcp_opt.push_str("{\n");
      tcp_opt.push_str("        unreachable!(\"failed to parse packet ");
      tcp_opt.push_str(&packet.name);
      tcp_opt.push_str(" with version {:?}\", version)\n");
      tcp_opt.push_str("      }\n");
    } else {
      tcp_opt.push_str(&id.to_string());
      tcp_opt.push_str(" => Self::");
      tcp_opt.push_str(&name);
      tcp_opt.push_str(" {\n");
      for (i, (field_name, _)) in field_names.iter().enumerate() {
        let from_tcp = &field_from_tcps[i];
        tcp_opt.push_str("        ");
        tcp_opt.push_str(field_name);
        tcp_opt.push_str(": ");
        tcp_opt.push_str(&from_tcp.to_string());
        tcp_opt.push_str(",\n");
      }
      tcp_opt.push_str("      },\n");
    }
    from_tcp_opts.push(tcp_opt);
  }
  let mut out = String::new();
  out.push_str("use crate::{\n");
  out.push_str("  math::Pos,\n");
  out.push_str("  net::tcp,\n");
  out.push_str("  proto,\n");
  out.push_str("  proto::packet_field::Type,\n");
  out.push_str("  version::ProtocolVersion,\n");
  out.push_str("  util::{nbt::NBT, UUID}\n");
  out.push_str("};\n");
  out.push_str("use std::collections::HashMap;\n");
  out.push_str("\n");
  out.push_str("/// Auto generated packet ids. This is a combination of all packet\n");
  out.push_str("/// names for all versions. Some of these packets are never used.\n");
  out.push_str("#[derive(Clone, Debug, PartialEq)]\n");
  out.push_str("pub enum Packet {\n");
  out.push_str("  None,\n");
  for k in kinds {
    out.push_str("  ");
    out.push_str(&k);
  }
  out.push_str("}\n");
  out.push_str("\n");
  out.push_str("impl Packet {\n");
  out.push_str("  /// Returns a GRPC specific id for this packet.\n");
  out.push_str("  pub fn id(&self) -> i32 {\n");
  out.push_str("    match self {\n");
  out.push_str("      Self::None => panic!(\"cannot get packet id of None packet\"),\n");
  for opt in id_opts {
    out.push_str("      ");
    out.push_str(&opt);
  }
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("\n");

  out.push_str("  /// Converts self into a protobuf\n");
  out.push_str("  pub fn to_proto(&self, version: ProtocolVersion) -> proto::Packet {\n");
  out.push_str("    match self {\n");
  out.push_str("      Self::None => panic!(\"cannot convert None packet to protobuf\"),\n");
  for opt in to_proto_opts {
    out.push_str("      ");
    out.push_str(&opt);
  }
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("  /// Converts the given protobuf into a packet\n");
  out.push_str("  pub fn from_proto(mut pb: proto::Packet, version: ProtocolVersion) -> Self {\n");
  out.push_str("    match pb.id {\n");
  for opt in from_proto_opts {
    out.push_str("      ");
    out.push_str(&opt);
  }
  out.push_str("      _ => Self::None\n");
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("\n");

  out.push_str("  /// Converts self into a tcp packet. This is used on the proxy to send packets to the client.\n");
  out.push_str("  pub fn to_tcp(&self, version: ProtocolVersion) -> tcp::Packet {\n");
  out.push_str("    match self {\n");
  out.push_str("      Self::None => panic!(\"cannot convert None packet to tcp\"),\n");
  for opt in to_tcp_opts {
    out.push_str("      ");
    out.push_str(&opt);
  }
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("\n");

  out.push_str("  /// Converts the given tcp packet into a grpc packet. This is used on the proxy to parse incoming packets.\n");
  out.push_str("  pub fn from_tcp(mut p: tcp::Packet, version: ProtocolVersion) -> Self {\n");
  out.push_str("    match to_grpc_id(p.id(), version) {\n");
  for opt in from_tcp_opts {
    out.push_str("      ");
    out.push_str(&opt);
  }
  out.push_str("      _ => Self::None,\n");
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("}\n");

  let mut from_grpc_id = String::new();
  let mut to_grpc_id = String::new();
  from_grpc_id.push_str("/// Converts a grpc packet id into a tcp packet id\n");
  from_grpc_id.push_str("pub fn from_grpc_id(id: i32, ver: ProtocolVersion) -> i32 {\n");
  from_grpc_id.push_str("  match ver {\n");
  to_grpc_id.push_str("/// Converts a tcp packet id into a grpc packet id\n");
  to_grpc_id.push_str("pub fn to_grpc_id(id: i32, ver: ProtocolVersion) -> i32 {\n");
  to_grpc_id.push_str("  match ver {\n");
  for (ver_name, ver) in versions.iter().sorted_by(|(ver_a, _), (ver_b, _)| ver_a.cmp(ver_b)) {
    from_grpc_id.push_str("    ProtocolVersion::");
    from_grpc_id.push_str(&ver_name.to_string().to_uppercase());
    from_grpc_id.push_str(" => match id {\n");
    to_grpc_id.push_str("    ProtocolVersion::");
    to_grpc_id.push_str(&ver_name.to_string().to_uppercase());
    to_grpc_id.push_str(" => match id {\n");
    let tcp_packets = if to_client { &ver.to_client } else { &ver.to_server };
    for (tcp_id, tcp_packet) in tcp_packets.iter().enumerate() {
      let grpc_id =
        packets.binary_search_by(|grpc_packet| grpc_packet.name.cmp(&tcp_packet.name)).unwrap();
      from_grpc_id.push_str("      ");
      from_grpc_id.push_str(&grpc_id.to_string());
      from_grpc_id.push_str(" => ");
      from_grpc_id.push_str(&tcp_id.to_string());
      from_grpc_id.push_str(",\n");
      to_grpc_id.push_str("      ");
      to_grpc_id.push_str(&tcp_id.to_string());
      to_grpc_id.push_str(" => ");
      to_grpc_id.push_str(&grpc_id.to_string());
      to_grpc_id.push_str(",\n");
    }
    from_grpc_id.push_str("      _ => panic!(\"unknown grpc id {}\", id),\n");
    from_grpc_id.push_str("    }\n");
    to_grpc_id.push_str("      _ => panic!(\"unknown tcp id {}\", id),\n");
    to_grpc_id.push_str("    }\n");
  }
  from_grpc_id.push_str("    ver => panic!(\"invalid version {:?}\", ver),\n");
  from_grpc_id.push_str("  }\n");
  from_grpc_id.push_str("}\n");
  to_grpc_id.push_str("    ver => panic!(\"invalid version {:?}\", ver),\n");
  to_grpc_id.push_str("  }\n");
  to_grpc_id.push_str("}\n");
  out.push_str(&from_grpc_id);
  out.push_str(&to_grpc_id);

  Ok(out)
}
