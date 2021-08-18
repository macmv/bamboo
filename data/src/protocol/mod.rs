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
    let mut to_client = HashSet::new();
    let mut to_server = HashSet::new();

    for (_, v) in versions {
      for p in v.to_client {
        to_client.insert(p.name);
      }
      for p in v.to_server {
        to_server.insert(p.name);
      }
    }
    // This is a custom packet. It is a packet sent from the proxy to the server,
    // which is used to authenticate the player.
    to_server.insert("Login".into());

    let to_client: Vec<String> = to_client.into_iter().sorted().collect();
    let to_server: Vec<String> = to_server.into_iter().sorted().collect();

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
        include!("cb.rs");
      }
      pub mod sb {
        #to_server
        include!("sb.rs");
      }
    })
  }
}

pub fn generate_packets(packets: &[String]) -> Result<TokenStream, Box<dyn Error>> {
  let mut kinds = vec![];
  for n in packets {
    kinds.push(Ident::new(&n.to_case(Case::Pascal), Span::call_site()));
  }
  let mut names = vec![];
  for n in packets {
    names.push(n);
  }
  let out = quote! {
    /// Auto generated packet ids. This is a combination of all packet
    /// names for all versions. Some of these packets are never used.
    #[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq, Hash)]
    pub enum ID {
      // We always want a None type, to signify an invalid packet
      None,
      #(#kinds,)*
    }
    impl ID {
      /// Parses the given string as a packet id. The string should be in
      /// snake case.
      pub fn parse_str(s: &str) -> Self {
        match s {
          #(#names => ID::#kinds,)*
          _ => ID::None,
        }
      }
    }
  };
  println!("{}", out.to_string());
  Ok(out)
}
