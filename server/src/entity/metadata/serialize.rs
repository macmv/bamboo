use super::{Field, Metadata};
use crate::item;
use common::version::ProtocolVersion;
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum MetadataError {
  Index(u8),
  Version(ProtocolVersion),
}

impl fmt::Display for MetadataError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::Index(v) => {
        write!(f, "cannot add item to metadata: invalid index (max is 254): {}", v)
      }
      Self::Version(v) => {
        write!(f, "cannot add item to metadata: invalid version: {:?}", v)
      }
    }
  }
}

impl Error for MetadataError {}

impl Metadata {
  /// Adds the short to the metadata. Only valid for 1.8
  pub fn set_short(&mut self, index: u8, value: i16) -> Result<(), MetadataError> {
    if self.ver != ProtocolVersion::V1_8 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Short(value))
  }
  /// Adds the int to the metadata. Only valid for 1.8
  pub fn set_int(&mut self, index: u8, value: i32) -> Result<(), MetadataError> {
    if self.ver != ProtocolVersion::V1_8 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Int(value))
  }
  /// Adds the int to the metadata. Only valid for 1.9+. This will be serialized
  /// as a varint.
  pub fn set_varint(&mut self, index: u8, value: i32) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Varint(value))
  }
  /// Adds the given bool to the metadata. Valid for 1.9+.
  pub fn set_bool(&mut self, index: u8, value: bool) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Bool(value))
  }

  /// Adds the given byte to the metadata. Valid for all versions.
  pub fn set_byte(&mut self, index: u8, value: u8) -> Result<(), MetadataError> {
    self.set_field(index, Field::Byte(value))
  }
  /// Adds the given float to the metadata. Valid for all versions.
  pub fn set_float(&mut self, index: u8, value: f32) -> Result<(), MetadataError> {
    self.set_field(index, Field::Float(value))
  }
  /// Adds the given string to the metadata. Valid for all versions.
  pub fn set_str(&mut self, index: u8, value: String) -> Result<(), MetadataError> {
    self.set_field(index, Field::String(value))
  }
  /// Adds the given item stack to the metadata. Valid for all versions.
  pub fn set_item(&mut self, index: u8, value: item::Stack) -> Result<(), MetadataError> {
    self.set_field(index, Field::Item(value))
  }

  fn set_field(&mut self, index: u8, value: Field) -> Result<(), MetadataError> {
    if index > 254 {
      return Err(MetadataError::Index(index));
    }
    self.fields.insert(index, value);
    Ok(())
  }
}
