mod json;
mod parse;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde_derive::{Deserialize, Serialize};
use std::{
  collections::{HashMap, HashSet},
  error::Error,
  fs,
  fs::File,
  io,
  io::Write,
  path::Path,
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

pub fn generate(dir: &Path) -> Result<TokenStream, Box<dyn Error>> {
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
    // Generates the packet id enum, for clientbound and serverbound packets
    let mut to_client = HashMap::new();
    let mut to_server = HashMap::new();

    for (_, v) in versions {
      for p in v.to_client {
        if !to_client.contains_key(&p.name) {
          to_client.insert(p.name.clone(), HashMap::new());
        }
        let fields = to_client.get_mut(&p.name).unwrap();
        for (name, field) in &p.fields {
          fields.insert(name.to_string(), field.clone());
        }
      }
      for p in v.to_server {
        if !to_server.contains_key(&p.name) {
          to_server.insert(p.name.clone(), HashMap::new());
        }
        let fields = to_server.get_mut(&p.name).unwrap();
        for (name, field) in &p.fields {
          fields.insert(name.to_string(), field.clone());
        }
      }
    }
    // This is a custom packet. It is a packet sent from the proxy to the server,
    // which is used to authenticate the player.
    to_server.insert("Login".into(), HashMap::new());

    let to_client: Vec<(String, HashMap<String, PacketField>)> =
      to_client.into_iter().sorted_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b)).collect();
    let to_server: Vec<(String, HashMap<String, PacketField>)> =
      to_server.into_iter().sorted_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b)).collect();

    let to_client = generate_packets(&to_client)?;
    let to_server = generate_packets(&to_server)?;

    // The include! trick is a terrible hack. It just lets met define both the enums
    // in one macro call, which allows for faster compilation.
    //
    // These files are going to be removed in the future, as I am going to move to
    // generating every packet as its own struct, so that fields are no longer
    // defined with strings.
    Ok(quote! {
      pub mod cb {
        #to_client
      }
      pub mod sb {
        #to_server
      }
    })
  }
}

impl PacketField {
  fn to_tokens(&self) -> TokenStream {
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

      Self::NBT => quote!(NBT),
      Self::OptionalNBT => quote!(Option<NBT>),
      Self::RestBuffer => quote!(Vec<u8>),
      Self::EntityMetadata => quote!(Vec<u8>), // Implemented on the server

      Self::Option(field) => {
        let inner = field.to_tokens();
        quote!(Option<#inner>)
      }
      Self::Array { count, value } => match count {
        CountType::Typed(_) | CountType::Named(_) => {
          let value = value.to_tokens();
          quote!(Vec<#value>)
        }
        CountType::Fixed(val) => {
          let value = value.to_tokens();
          quote!([#value; #val])
        }
      },
      _ => quote!(Vec<u8>),
    }
  }
}

pub fn generate_packets(
  packets: &[(String, HashMap<String, PacketField>)],
) -> Result<TokenStream, Box<dyn Error>> {
  let mut kinds = vec![];
  let mut to_proto_opts = vec![];
  let mut id_opts = vec![];
  for (id, (n, fields)) in packets.into_iter().enumerate() {
    let name = Ident::new(&n.to_case(Case::Pascal), Span::call_site());
    let mut field_names = vec![];
    let mut field_tys = vec![];
    for (field_name, field_val) in fields {
      let mut field_name = field_name.to_string();
      // Avoid keyword conflicts
      if field_name == "type" {
        field_name = "type_".to_string();
      }
      field_names.push(Ident::new(&field_name, Span::call_site()));
      field_tys.push(field_val.to_tokens());
    }
    kinds.push(quote! {
      #name {
        #(#field_names: #field_tys),*
      }
    });
    to_proto_opts.push(quote! {
      Self::#name {
        #(#field_names),*
      } => {
        // TODO: Fill in fields
        proto::Packet {}
      }
    });
    id_opts.push(quote! {
      Self::#name { .. } => { #id }
    });
  }
  let mut names = vec![];
  for (n, _) in packets {
    names.push(n);
  }
  let out = quote! {
    use num_derive::ToPrimitive;
    use crate::{
      math::Pos,
      proto,
      util::{nbt::NBT, UUID},
    };
    /// Auto generated packet ids. This is a combination of all packet
    /// names for all versions. Some of these packets are never used.
    #[derive(Clone, Debug, PartialEq)]
    pub enum Packet {
      // We always want a None type, to signify an invalid packet
      None,
      #(#kinds,)*
    }
    impl Packet {
      /// Returns a GRPC specific id for this packet.
      pub fn id(&self) -> i32 {
        match self {
          None => panic!("cannot get packet id of None packet"),
          #(#id_opts)*,
        }
      }
      /// Converts self into a protobuf
      pub fn to_proto(&self) -> proto::Packet {
        match self {
          None => panic!("cannot convert None packet to protobuf"),
          #(#to_proto_opts)*,
        }
      }
    }
  };
  println!("{}", out.to_string());
  Ok(out)
}
