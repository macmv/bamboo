mod json;
mod parse;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde_derive::{Deserialize, Serialize};
use std::{
  collections::HashMap, error::Error, fmt::Write, fs, fs::File, io::Write as _, path::Path,
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

struct VersionedPacket {
  name:        String,
  field_names: HashMap<String, usize>,
  fields:      Vec<(String, PacketField)>,
}

impl VersionedPacket {
  fn new(name: String) -> Self {
    VersionedPacket { name, field_names: HashMap::new(), fields: vec![] }
  }

  fn add_version(&mut self, ver: &str, packet: Packet) {
    for (name, mut field) in packet.fields {
      if let Some(&idx) = self.field_names.get(&name) {
        let existing = &mut self.fields[idx].1;
        if existing != &field {
          existing.extend_to_fit(&packet.name, &name, &mut field);
        }
      } else {
        self.field_names.insert(name.clone(), self.fields.len());
        self.fields.push((name, field));
      }
    }
  }

  fn field_name_tys(&self) -> Vec<(String, String)> {
    let mut vals = vec![];
    for (name, field) in &self.fields {
      vals.push((self.convert_name(name).to_string(), field.ty_lit().to_string()));
    }
    vals
  }
  fn field_tys(&self) -> Vec<TokenStream> {
    let mut tys = vec![];
    for (_, field) in &self.fields {
      tys.push(field.ty_lit());
    }
    tys
  }
  fn field_ty_enums(&self) -> Vec<TokenStream> {
    let mut tys = vec![];
    for (_, field) in &self.fields {
      tys.push(field.ty_enum());
    }
    tys
  }
  fn field_ty_keys(&self) -> Vec<TokenStream> {
    let mut tys = vec![];
    for (_, field) in &self.fields {
      tys.push(field.ty_key());
    }
    tys
  }
  fn field_values(&self) -> Vec<TokenStream> {
    let mut vals = vec![];
    for (name, field) in &self.fields {
      vals.push(field.generate_conversion(self.convert_name(name)));
    }
    vals
  }
  fn name(&self) -> &str {
    &self.name
  }

  fn convert_name<'a>(&self, name: &'a str) -> &'a str {
    // Avoid keyword conflicts
    if name == "type" {
      "type_"
    } else {
      name
    }
  }
}

impl PacketField {
  fn extend_to_fit(&mut self, packet_name: &str, field_name: &str, other: &mut PacketField) {
    let valid_a = self.extend_to_fit_inner(other);
    let valid_b = other.extend_to_fit_inner(self);
    if !valid_a && !valid_b {
      eprintln!(
        "differing field types on packet `{}`, with field `{}` (got {:?} and {:?})",
        packet_name, field_name, self, other
      );
    }
  }

  fn extend_to_fit_inner(&mut self, other: &mut PacketField) -> bool {
    match self {
      Self::Bool if other == &PacketField::Int(IntType::U8) => *other = PacketField::Bool,
      Self::Int(IntType::VarInt) if other == &PacketField::Int(IntType::U8) => {
        *other = PacketField::Int(IntType::VarInt)
      }
      Self::Int(IntType::VarInt) if other == &PacketField::Int(IntType::I8) => {
        *other = PacketField::Int(IntType::VarInt)
      }
      Self::Int(IntType::I32) if other == &PacketField::Int(IntType::I8) => {
        *other = PacketField::Int(IntType::I32)
      }
      _ => return false,
    }
    true
  }
}

fn to_versioned(
  versions: HashMap<String, Version>,
) -> (Vec<VersionedPacket>, Vec<VersionedPacket>) {
  // Generates the packet id enum, for clientbound and serverbound packets
  let mut to_client = HashMap::new();
  let mut to_server = HashMap::new();

  for (version, v) in versions {
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
      Packet { name: "Login".into(), field_names: HashMap::new(), fields: vec![] },
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
  fn generate_conversion(&self, name: &str) -> TokenStream {
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
}

fn generate_packets(packets: Vec<VersionedPacket>) -> Result<String, Box<dyn Error>> {
  let mut kinds = vec![];
  // let mut to_proto_opts = vec![];
  // let mut id_opts = vec![];
  for (id, packet) in packets.into_iter().enumerate() {
    let id = id as i32;
    let name = packet.name().to_case(Case::Pascal);
    let field_ty_enums = packet.field_ty_enums();
    let field_ty_keys = packet.field_ty_keys();
    let field_values = packet.field_values();
    let mut kind = String::new();
    kind.push_str("  ");
    kind.push_str(&name);
    kind.push_str(" {\n");
    for (name, ty) in packet.field_name_tys() {
      kind.push_str("    ");
      kind.push_str(&name);
      kind.push_str(": ");
      kind.push_str(&ty);
      kind.push_str(",\n");
    }
    kind.push_str("  },\n");
    kinds.push(kind);
    // to_proto_opts.push(quote! {
    //   Self::#name {
    //     #(#field_names),*
    //   } => {
    //     let mut fields = HashMap::new();
    //     #(
    //       fields.insert(#field_name_strs.to_string(), proto::PacketField {
    //         ty: Type::#field_ty_enums.into(),
    //         #field_ty_keys: #field_values,
    //         ..Default::default()
    //       });
    //     )*
    //     proto::Packet {
    //       id: #id,
    //       fields,
    //       other: None,
    //     }
    //   }
    // });
    // id_opts.push(quote! {
    //   Self::#name { .. } => { #id }
    // });
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
    out.push_str(&k);
  }
  out.push_str("}\n");

  // let out = quote! {
  //   use crate::{
  //     math::Pos,
  //     proto,
  //     proto::packet_field::Type,
  //     util::{nbt::NBT, UUID},
  //   };
  //   use std::collections::HashMap;
  //   /// Auto generated packet ids. This is a combination of all packet
  //   /// names for all versions. Some of these packets are never used.
  //   #[derive(Clone, Debug, PartialEq)]
  //   pub enum Packet {
  //     // We always want a None type, to signify an invalid packet
  //     None,
  //     #(#kinds,)*
  //   }
  //   impl Packet {
  //     /// Returns a GRPC specific id for this packet.
  //     pub fn id(&self) -> i32 {
  //       match self {
  //         Self::None => panic!("cannot get packet id of None packet"),
  //         #(#id_opts)*,
  //       }
  //     }
  //     /// Converts self into a protobuf
  //     pub fn to_proto(&self) -> proto::Packet {
  //       match self {
  //         Self::None => panic!("cannot convert None packet to protobuf"),
  //         #(#to_proto_opts)*,
  //       }
  //     }
  //   }
  // };
  // Will save the output to disk
  // let mut p = std::process::Command::new("rustfmt")
  //   .stdin(std::process::Stdio::piped())
  //   .stdout(
  //     std::fs::File::create("/home/macmv/Desktop/Programming/rust/sugarcane/
  // common/src/net/cb.rs")       .unwrap(),
  //   )
  //   .spawn()
  //   .unwrap();
  // std::io::Write::write_all(p.stdin.as_mut().unwrap(),
  // out.to_string().as_bytes()).unwrap(); p.wait_with_output().unwrap();
  // Will print the output
  // let mut p =
  //   std::process::Command::new("rustfmt").stdin(std::process::Stdio::piped()).
  // spawn().unwrap(); std::io::Write::write_all(p.stdin.as_mut().unwrap(),
  // out.to_string().as_bytes()).unwrap(); p.wait_with_output().unwrap();

  Ok(out)
}
