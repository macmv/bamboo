mod json;
mod parse;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, fs, fs::File, io::Write, path::Path};

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
  pub fn into_compare(self) -> Option<String> {
    match self {
      Self::CompareTo(v) => Some(v),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Packet {
  pub name:        String,
  // Can be used to lookup a field by name
  pub field_names: HashMap<String, usize>,
  pub fields:      Vec<(String, PacketField)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
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
  version_names: HashMap<String, usize>,
  versions:      Vec<(String, PacketField)>,
}

struct VersionedPacket {
  name:        String,
  field_names: HashMap<String, usize>,
  // Map of versions to fields
  fields:      Vec<VersionedField>,
}

impl VersionedField {
  fn new(ver: &str, name: String, field: PacketField) -> Self {
    let mut version_names = HashMap::new();
    version_names.insert(ver.to_string(), 0);
    VersionedField { name, version_names, versions: vec![(ver.to_string(), field)] }
  }
  fn add_ver(&mut self, ver: &str, field: PacketField) {
    self.version_names.insert(ver.to_string(), self.versions.len());
    self.versions.push((ver.to_string(), field));
  }
  fn latest(&self) -> &PacketField {
    &self.versions.last().unwrap().1
  }
  fn add_all(&self, out: &mut Vec<NamedPacketField>) {
    if self.versions.len() == 1 {
      let mut name = self.name.clone();
      // Avoid keyword conflicts
      if name == "type" {
        name = "type_".to_string();
      }
      out.push(NamedPacketField { name, field: self.versions.first().unwrap().1.clone() });
    } else {
      for (ver, field) in &self.versions {
        let mut name = self.name.clone();
        name.push_str("_");
        name.push_str(&ver.to_lowercase());
        out.push(NamedPacketField { name, field: field.clone() });
      }
    }
  }
}

impl VersionedPacket {
  fn new(name: String) -> Self {
    VersionedPacket { name, field_names: HashMap::new(), fields: vec![] }
  }

  fn add_version(&mut self, ver: &str, packet: Packet) {
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

  fn field_name_tys(&self) -> Vec<(String, String)> {
    let mut vals = vec![];
    for field in self.fields() {
      vals.push((field.name.clone(), field.field.ty_lit().to_string()));
    }
    vals
  }
  fn field_tys(&self) -> Vec<TokenStream> {
    let mut tys = vec![];
    for field in self.fields() {
      tys.push(field.field.ty_lit());
    }
    tys
  }
  fn field_ty_enums(&self) -> Vec<TokenStream> {
    let mut tys = vec![];
    for field in self.fields() {
      tys.push(field.field.ty_enum());
    }
    tys
  }
  fn field_ty_keys(&self) -> Vec<TokenStream> {
    let mut tys = vec![];
    for field in self.fields() {
      tys.push(field.field.ty_key());
    }
    tys
  }
  fn field_to_protos(&self) -> Vec<TokenStream> {
    let mut vals = vec![];
    for field in self.fields() {
      vals.push(field.field.generate_to_proto(&field.name));
    }
    vals
  }
  fn field_from_protos(&self) -> Vec<TokenStream> {
    let mut vals = vec![];
    for field in self.fields() {
      vals.push(field.field.generate_from_proto(&field.name));
    }
    vals
  }
  fn name(&self) -> &str {
    &self.name
  }
}

fn to_versioned(
  versions: HashMap<String, Version>,
) -> (Vec<VersionedPacket>, Vec<VersionedPacket>) {
  // Generates the packet id enum, for clientbound and serverbound packets
  let mut to_client = HashMap::new();
  let mut to_server = HashMap::new();

  for (version, v) in versions.into_iter().sorted_by(|(ver_a, _), (ver_b, _)| {
    let major_a: i32 = ver_a.split("_").nth(1).unwrap().parse().unwrap();
    let major_b: i32 = ver_b.split("_").nth(1).unwrap().parse().unwrap();
    if major_a == major_b {
      let minor_a = ver_a.split("_").nth(2).map(|v| v.parse().unwrap()).unwrap_or(0); // for example, 1.15 is the same as 1.15.0
      let minor_b = ver_b.split("_").nth(2).map(|v| v.parse().unwrap()).unwrap_or(0);
      minor_a.cmp(&minor_b)
    } else {
      major_a.cmp(&major_b)
    }
  }) {
    for p in v.to_client {
      if !to_client.contains_key(&p.name) {
        to_client.insert(p.name.clone(), VersionedPacket::new(p.name.clone()));
      }
      to_client.get_mut(&p.name).unwrap().add_version(&version, p);
    }
    for p in v.to_server {
      if !to_server.contains_key(&p.name) {
        to_server.insert(p.name.clone(), VersionedPacket::new(p.name.clone()));
      }
      to_server.get_mut(&p.name).unwrap().add_version(&version, p);
    }
    if !to_server.contains_key("Login") {
      to_server.insert("Login".into(), VersionedPacket::new("Login".into()));
    }
    to_server.get_mut("Login").unwrap().add_version(
      &version,
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
  let versions = parse::load_all(&prismarine_path.join("data/pc"))?;

  fs::create_dir_all(&dir)?;
  {
    // Generates the version json in a much more easily read format. This is much
    // faster to compile than generating source code.
    let mut f = File::create(&dir.join("versions.json"))?;
    writeln!(f, "{}", serde_json::to_string(&versions)?)?;
  }
  {
    let (to_client, to_server) = to_versioned(versions);
    let to_client = generate_packets(to_client)?;
    let to_server = generate_packets(to_server)?;

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
  fn generate_to_proto(&self, name: &str) -> TokenStream {
    let name = Ident::new(name, Span::call_site());
    match self {
      Self::Bool => quote!(*#name),
      Self::Int(ity) => match ity {
        IntType::I8 => quote!((*#name as u8).into()),
        IntType::U8 => quote!((*#name).into()),
        IntType::I16 => quote!((*#name as u16).into()),
        IntType::U16 => quote!((*#name).into()),
        IntType::I32 => quote!(*#name),
        IntType::I64 => quote!(*#name as u64),
        IntType::VarInt => quote!(*#name),
        IntType::OptVarInt => quote!(#name.unwrap_or(0)),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => quote!(*#name),
        FloatType::F64 => quote!(*#name),
      },
      Self::UUID => quote!(Some(#name.as_proto())),
      Self::String => quote!(#name.to_string()),
      Self::Position => quote!(#name.to_u64()),

      // Self::NBT => quote!(#name.clone()),
      // Self::OptionalNBT => quote!(#name.clone()),
      // Self::RestBuffer => quote!(#name.clone()),
      // Self::EntityMetadata => quote!(#name.clone()), // Implemented on the server

      // Self::Option(field) => quote!(#name.unwrap()),
      _ => quote!(#name.clone()),
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
}

fn generate_packets(packets: Vec<VersionedPacket>) -> Result<String, Box<dyn Error>> {
  let mut kinds = vec![];
  let mut id_opts = vec![];
  let mut to_proto_opts = vec![];
  let mut from_proto_opts = vec![];
  for (id, packet) in packets.into_iter().enumerate() {
    let id = id as i32;
    let name = packet.name().to_case(Case::Pascal);
    let field_names = packet.field_name_tys();
    let field_ty_enums = packet.field_ty_enums();
    let field_ty_keys = packet.field_ty_keys();
    let field_to_protos = packet.field_to_protos();
    let field_from_protos = packet.field_from_protos();
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
    proto_opt.push_str(" => Self::");
    proto_opt.push_str(&name);
    proto_opt.push_str(" {\n");
    for (i, (field_name, _)) in field_names.iter().enumerate() {
      let from_proto = &field_from_protos[i];
      proto_opt.push_str("        ");
      proto_opt.push_str(field_name);
      proto_opt.push_str(": ");
      proto_opt.push_str(&from_proto.to_string());
      // proto_opt.push_str(": pb.fields[\"");
      // proto_opt.push_str(field_name);
      // proto_opt.push_str("\"].");
      // proto_opt.push_str(&ty_key.to_string());
      proto_opt.push_str(",\n");
    }
    proto_opt.push_str("      },\n");
    from_proto_opts.push(proto_opt);
  }
  let mut out = String::new();
  out.push_str("use crate::{\n");
  out.push_str("  math::Pos,\n");
  out.push_str("  proto,\n");
  out.push_str("  proto::packet_field::Type,\n");
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
  out.push_str("  pub fn to_proto(&self) -> proto::Packet {\n");
  out.push_str("    match self {\n");
  out.push_str("      Self::None => panic!(\"cannot convert None packet to protobuf\"),\n");
  for opt in to_proto_opts {
    out.push_str("      ");
    out.push_str(&opt);
  }
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("  /// Converts the given protobuf into a packet\n");
  out.push_str("  pub fn from_proto(mut pb: proto::Packet) -> Self {\n");
  out.push_str("    match pb.id {\n");
  for opt in from_proto_opts {
    out.push_str("      ");
    out.push_str(&opt);
  }
  out.push_str("      _ => Self::None\n");
  out.push_str("    }\n");
  out.push_str("  }\n");
  out.push_str("}\n");

  Ok(out)
}
