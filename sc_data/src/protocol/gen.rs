//! Functions on PacketField and VersionedField to generate the protocol
//! readers/writers.

use super::{FloatType, IntType, NamedPacketField, PacketField, Version, VersionedField};
use std::collections::HashMap;

impl PacketField {
  pub fn ty_lit(&self, packet: &str) -> String {
    match self {
      Self::Bool => "bool".into(),
      Self::Int(ity) => match ity {
        IntType::I8 => "i8".into(),
        IntType::U8 => "u8".into(),
        IntType::I16 => "i16".into(),
        IntType::U16 => "u16".into(),
        IntType::I32 => "i32".into(),
        IntType::I64 => "i64".into(),
        IntType::VarInt => "i32".into(),
        IntType::OptVarInt => "i32".into(), // TODO: Might want to change this to Option<i32>
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => "f32".into(),
        FloatType::F64 => "f64".into(),
      },
      Self::UUID => "UUID".into(),
      Self::String => "String".into(),
      Self::Position => "Pos".into(),

      // Self::NBT => "NBT",
      // Self::OptionalNBT => "Option<NBT>",
      // Self::RestBuffer => "Vec<u8>",
      // Self::EntityMetadata => "Vec<u8>", // Implemented on the server

      // Self::Option(field) => {
      //   let inner = field.ty_lit;
      //   "Option<#inner>"
      // }
      // Self::Array { count, value } => match count {
      //   CountType::Typed"_) | CountType::Named"_) => {
      //     let value = value.ty_lit");
      //     "Vec<#value>)
      //   }
      //   CountType::Fixed"len) => {
      //     let value = value.ty_lit");
      //     "[#value; #len])
      //   }
      // },
      Self::DefinedType(name) => match name.as_str() {
        "slot" => "Item".into(),
        "tags" => "Vec<u8>".into(),
        _ => panic!("undefined field type {}", name),
      },
      Self::Switch { compare_to: _, fields, default: _ } => {
        if packet == "use_entity" {
          format!("Option<{}>", fields.iter().next().unwrap().1.ty_lit(packet))
        } else {
          "Vec<u8>".into()
        }
      }
      _ => "Vec<u8>".into(),
    }
  }
  pub fn generate_to_sc(&self, val: &str) -> String {
    match self {
      Self::Bool => format!("m.write_bool(*{})", val),
      Self::Int(ity) => match ity {
        IntType::I8 => format!("m.write_i8(*{})", val),
        IntType::U8 => format!("m.write_u8(*{})", val),
        IntType::I16 => format!("m.write_i16(*{})", val),
        IntType::U16 => format!("m.write_u16(*{})", val),
        IntType::I32 => format!("m.write_i32(*{})", val),
        IntType::I64 => format!("m.write_i64(*{})", val),
        IntType::VarInt => format!("m.write_i32(*{})", val),
        IntType::OptVarInt => format!("m.write_i32({}.unwrap_or(0))", val),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => format!("m.write_f32(*{})", val),
        FloatType::F64 => format!("m.write_f64(*{})", val),
      },
      Self::UUID => format!("m.write_bytes(&{}.as_be_bytes())", val),
      Self::String => format!("m.write_str({})", val),
      Self::Position => format!("m.write_u64({}.to_u64())", val),

      // Self::NBT => format!(#name.clone()),
      // Self::OptionalNBT => format!(#name.clone()),
      // Self::RestBuffer => format!(#name.clone()),
      // Self::EntityMetadata => format!(#name.clone()), // Implemented on the server

      // Self::Option(field) => format!(#name.unwrap()),
      Self::DefinedType(name) => match name.as_str() {
        "slot" => format!("{}.to_sc(&mut m)", val),
        "tags" => format!("m.write_buf({})", val),
        _ => panic!("undefined field type {}", name),
      },
      _ => format!("m.write_buf({})", val),
    }
  }
  /// Takes a field name and packet name, in order to generate errors.
  pub fn generate_from_sc(&self, field: &str, packet: &str) -> String {
    let err = &format!(".map_err(|e| (\"{}\", \"{}\", e))?", packet, field);
    match self {
      Self::Bool => format!("m.read_bool(){}", err),
      Self::Int(ity) => match ity {
        IntType::I8 => format!("m.read_i8(){}", err),
        IntType::U8 => format!("m.read_u8(){}", err),
        IntType::I16 => format!("m.read_i16(){}", err),
        IntType::U16 => format!("m.read_u16(){}", err),
        IntType::I32 => format!("m.read_i32(){}", err),
        IntType::I64 => format!("m.read_i64(){}", err),
        IntType::VarInt => format!("m.read_i32(){}", err),
        IntType::OptVarInt => format!("Some(m.read_i32(){})", err),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => format!("m.read_f32(){}", err),
        FloatType::F64 => format!("m.read_f64(){}", err),
      },
      Self::UUID => format!("UUID::from_bytes(m.read_bytes(16){}.try_into().unwrap())", err),
      Self::String => format!("m.read_str(){}", err),
      Self::Position => format!("Pos::from_u64(m.read_u64(){})", err),

      // Self::NBT => (#name.clone()),
      // Self::OptionalNBT => (#name.clone()),
      // Self::RestBuffer => (#name.clone()),
      // Self::EntityMetadata => (#name.clone()), // Implemented on the server

      // Self::Option(field) => (#name.unwrap()),
      Self::DefinedType(name) => match name.as_str() {
        "slot" => format!("Item::from_sc(&mut m){}", err),
        "tags" => format!("m.read_buf(){}", err),
        _ => panic!("undefined field type {}", name),
      },
      _ => format!("m.read_buf(){}", err),
    }
  }
  pub fn generate_to_tcp(&self, val: &str) -> String {
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
      Self::DefinedType(name) => match name.as_str() {
        "slot" => format!("out.write_item({})", val),
        "tags" => format!("out.write_buf({})", val),
        _ => panic!("undefined field type {}", name),
      },
      _ => format!("out.write_buf({})", val),
    }
  }
  pub fn generate_from_tcp(&self, packet: &str) -> String {
    match self {
      Self::Bool => "p.read_bool()".into(),
      Self::Int(ity) => match ity {
        IntType::I8 => "p.read_i8()".into(),
        IntType::U8 => "p.read_u8()".into(),
        IntType::I16 => "p.read_i16()".into(),
        IntType::U16 => "p.read_u16()".into(),
        IntType::I32 => "p.read_i32()".into(),
        IntType::I64 => "p.read_i64()".into(),
        IntType::VarInt => "p.read_varint()".into(),
        IntType::OptVarInt => "Some(p.read_varint())".into(),
      },
      Self::Float(fty) => match fty {
        FloatType::F32 => "p.read_f32()".into(),
        FloatType::F64 => "p.read_f64()".into(),
      },
      Self::UUID => "p.read_uuid()".into(),
      Self::String => "p.read_str()".into(),
      Self::Position => "p.read_pos()".into(),

      // Self::NBT => (#name.clone()),
      // Self::OptionalNBT => (#name.clone()),
      // Self::RestBuffer => (#name.clone()),
      // Self::EntityMetadata => (#name.clone()), // Implemented on the server

      // Self::Option(field) => (#name.unwrap()),
      Self::DefinedType(name) => match name.as_str() {
        "slot" => "p.read_item()".into(),
        "tags" => "{ let len = p.read_varint(); p.read_buf(len) }".into(),
        "void" => "None".into(),
        _ => panic!("undefined field type {}", name),
      },
      Self::Switch { compare_to, fields, default } => {
        if packet == "use_entity" {
          let mut out = format!("match {} {{\n", compare_to);
          for (k, v) in fields {
            out += &format!("          {} => Some({}),\n", k, v.generate_from_tcp(packet));
          }
          out += &format!("          _ => {}\n", default.generate_from_tcp(packet));
          out += "        }";
          out
        } else {
          "{ let len = p.read_varint(); p.read_buf(len) }".into()
        }
      }
      _ => "{ let len = p.read_varint(); p.read_buf(len) }".into(),
    }
  }
}

impl VersionedField {
  pub fn new(ver: Version, name: String, field: PacketField) -> Self {
    let mut version_names = HashMap::new();
    version_names.insert(ver, 0);
    VersionedField { name, version_names, versions: vec![(ver, field)], removed_in: None }
  }
  pub fn add_ver(&mut self, ver: Version, field: PacketField) {
    self.version_names.insert(ver, self.versions.len());
    self.versions.push((ver, field));
  }
  pub fn latest(&self) -> &PacketField {
    &self.versions.last().unwrap().1
  }
  pub fn set_removed_version(&mut self, ver: Version) {
    if self.removed_in.is_none() {
      self.removed_in = Some(ver);
    }
  }
  pub fn add_all(&self, out: &mut Vec<NamedPacketField>) {
    if self.versions.len() == 1 {
      // If the first version is not 1.8, we need this to be a multi versioned field
      let mut multi_versioned = self.versions.first().unwrap().0 != Version { major: 8, minor: 0 };
      let mut name = self.name.clone();
      if multi_versioned {
        name.push_str("_");
        name.push_str(&self.versions.first().unwrap().0.to_string());
      }
      if let Some(ver) = self.removed_in {
        multi_versioned = true;
        name.push_str("_removed_");
        name.push_str(&ver.to_string());
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
  pub fn multi_versioned(&self) -> bool {
    self.versions.len() > 1
      || self.versions.first().unwrap().0 != Version { major: 8, minor: 0 }
      || self.removed_in.is_some()
  }
  pub fn add_all_ver(&self, out: &mut Vec<(bool, NamedPacketField)>, matching_ver: Version) {
    if self.versions.len() == 1 {
      // If the first version is not 1.8, we need this to be a multi versioned field
      let mut multi_versioned = self.versions.first().unwrap().0 != Version { major: 8, minor: 0 };
      let mut name = self.name.clone();
      if multi_versioned {
        name.push_str("_");
        name.push_str(&self.versions.first().unwrap().0.to_string());
      }
      // Make sure that we don't set is_ver to true for a field that hasn't been added
      // to this packet yet.
      let mut is_ver = matching_ver >= self.versions.first().unwrap().0;
      if let Some(ver) = self.removed_in {
        multi_versioned = true;
        if matching_ver >= ver {
          is_ver = false;
        }
        name.push_str("_removed_");
        name.push_str(&ver.to_string());
      }
      // Avoid keyword conflicts
      if name == "type" {
        name = "type_".to_string();
      }
      out.push((
        is_ver,
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
