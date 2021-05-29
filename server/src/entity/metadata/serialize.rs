use super::{Field, Metadata, Pose};
use crate::item;
use common::{
  math::{BlockDirection, Pos},
  util::{Chat, UUID},
  version::ProtocolVersion,
};
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
  /// Adds the given position to the metadata. Valid for all versions.
  pub fn set_pos(&mut self, index: u8, value: Pos) -> Result<(), MetadataError> {
    self.set_field(index, Field::Position(value))
  }
  /// Adds the given rotation to the metadata. Valid for all versions.
  pub fn set_rotation(&mut self, index: u8, x: f32, y: f32, z: f32) -> Result<(), MetadataError> {
    self.set_field(index, Field::Rotation(x, y, z))
  }

  /// Adds the int to the metadata. Only valid for 1.9+. This will be serialized
  /// as a varint.
  pub fn set_varint(&mut self, index: u8, value: i32) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Varint(value))
  }
  /// Adds the given chat to the metadata. Valid for 1.9+.
  pub fn set_chat(&mut self, index: u8, value: Chat) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Chat(value))
  }
  /// Adds the given bool to the metadata. Valid for 1.9+.
  pub fn set_bool(&mut self, index: u8, value: bool) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Bool(value))
  }
  /// Adds the given optional position to the metadata. Valid for 1.9+.
  pub fn set_opt_pos(&mut self, index: u8, value: Option<Pos>) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::OptPosition(value))
  }
  /// Adds the given block direction to the metadata. Valid for 1.9+.
  pub fn set_dir(&mut self, index: u8, value: BlockDirection) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Direction(value))
  }
  /// Adds the given optional uuid to the metadata. Valid for 1.9+.
  pub fn set_opt_uuid(&mut self, index: u8, value: Option<UUID>) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::OptUUID(value))
  }
  /// Adds the given block id to the metadata. Valid for 1.9+.
  pub fn set_block_id(&mut self, index: u8, value: i32) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_9 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::BlockID(value))
  }

  /// Adds the given nbt data to the metadata. Valid for 1.12+.
  pub fn set_nbt(&mut self, index: u8, value: Vec<u8>) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_12 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::NBT(value))
  }

  /// Adds the given optional chat to the metadata. Valid for 1.13+.
  pub fn set_opt_chat(&mut self, index: u8, value: Option<Chat>) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_13 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::OptChat(value))
  }
  /// Adds the given nbt data to the metadata. Valid for 1.13+.
  pub fn set_particle(&mut self, index: u8, value: Vec<u8>) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_13 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Particle(value))
  }

  /// Adds the given nbt data to the metadata. Valid for 1.14+.
  ///
  /// `ty` is the villager type, `profession` is the villager's profession, and
  /// `level` is the villager's level.
  pub fn set_villager(
    &mut self,
    index: u8,
    ty: i32,
    profession: i32,
    level: i32,
  ) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_14 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::VillagerData(ty, profession, level))
  }
  /// Adds the given optional varint to the metadata. Valid for 1.14+.
  pub fn set_opt_varint(&mut self, index: u8, value: Option<i32>) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_14 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::OptVarint(value))
  }
  /// Adds the given pos to the metadata. Valid for 1.14+.
  pub fn set_pose(&mut self, index: u8, value: Pose) -> Result<(), MetadataError> {
    if self.ver < ProtocolVersion::V1_14 {
      return Err(MetadataError::Version(self.ver));
    }
    self.set_field(index, Field::Pose(value))
  }

  fn set_field(&mut self, index: u8, value: Field) -> Result<(), MetadataError> {
    if index > 254 {
      return Err(MetadataError::Index(index));
    }
    self.fields.insert(index, value);
    Ok(())
  }
}
