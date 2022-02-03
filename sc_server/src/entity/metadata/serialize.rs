use super::{Field, Metadata, Pose};
use crate::item;
use sc_common::{
  math::{Face, Pos},
  util::{Buffer, Chat, UUID},
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
  pub fn set_dir(&mut self, index: u8, value: Face) -> Result<(), MetadataError> {
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

  /// Serializes the entity metadata. This will not consume the metadata, and
  /// will never fail. The `set` functions are where error handling is done.
  pub fn serialize(&self) -> Vec<u8> {
    let mut out = Buffer::new(vec![]);
    for (id, field) in &self.fields {
      if self.ver == ProtocolVersion::V1_8 {
        // Index and type are the same byte in 1.8
        let mut index_type = id & 0x1f;
        match field {
          Field::Byte(_) => index_type |= 0 << 5,
          Field::Short(_) => index_type |= 1 << 5,
          Field::Int(_) => index_type |= 2 << 5,
          Field::Float(_) => index_type |= 3 << 5,
          Field::String(_) => index_type |= 4 << 5,
          Field::Item(_) => index_type |= 5 << 5,
          Field::Position(_) => index_type |= 6 << 5,
          Field::Rotation(_, _, _) => index_type |= 7 << 5,
          _ => unreachable!(),
        }
        out.write_u8(index_type);
        match field {
          Field::Byte(v) => out.write_u8(*v),
          Field::Short(v) => out.write_i16(*v),
          Field::Int(v) => out.write_i32(*v),
          Field::Float(v) => out.write_f32(*v),
          Field::String(v) => out.write_str(v),
          Field::Item(v) => {
            out.write_i16(v.item().id() as i16);
            out.write_u8(v.amount());
            out.write_i16(0); // Item damage
            out.write_u8(0x00); // TODO: NBT
          }
          Field::Position(v) => {
            out.write_i32(v.x());
            out.write_i32(v.y());
            out.write_i32(v.z());
          }
          Field::Rotation(x, y, z) => {
            out.write_f32(*x);
            out.write_f32(*y);
            out.write_f32(*z);
          }
          _ => unreachable!(),
        }
      } else {
        out.write_varint((*id).into());
        // Thank you minecraft. All of this is just for the metadata types.
        out.write_u8(match field {
          Field::Byte(_) => 0,
          Field::Varint(_) => 1,
          Field::Float(_) => 2,
          Field::String(_) => 3,
          Field::Chat(_) => 5,
          _ => {
            if self.ver >= ProtocolVersion::V1_13 {
              match field {
                Field::OptChat(_) => 5,
                Field::Item(_) => 6,
                Field::Bool(_) => 7,
                Field::Rotation(_, _, _) => 8,
                Field::Position(_) => 9,
                Field::OptPosition(_) => 10,
                Field::Direction(_) => 11,
                Field::OptUUID(_) => 12,
                Field::BlockID(_) => 13,
                Field::NBT(_) => 14,
                Field::Particle(_) => 15,
                _ => {
                  if self.ver >= ProtocolVersion::V1_14 {
                    match field {
                      Field::VillagerData(_, _, _) => 16,
                      Field::OptVarint(_) => 17,
                      Field::Pose(_) => 18,
                      _ => unreachable!(),
                    }
                  } else {
                    unreachable!()
                  }
                }
              }
            } else {
              match field {
                Field::Item(_) => 5,
                Field::Bool(_) => 6,
                Field::Rotation(_, _, _) => 7,
                Field::Position(_) => 8,
                Field::OptPosition(_) => 9,
                Field::Direction(_) => 10,
                Field::OptUUID(_) => 11,
                Field::BlockID(_) => 12,
                _ => {
                  if self.ver == ProtocolVersion::V1_12 {
                    match field {
                      Field::NBT(_) => 13,
                      _ => unreachable!(),
                    }
                  } else {
                    unreachable!()
                  }
                }
              }
            }
          }
        });
        match field {
          Field::Short(_) => unreachable!(),
          Field::Int(_) => unreachable!(),
          Field::Byte(v) => out.write_u8(*v),
          Field::Varint(v) => out.write_varint(*v),
          Field::Float(v) => out.write_f32(*v),
          Field::String(v) => out.write_str(v),
          Field::Chat(v) => out.write_str(&v.to_json()),
          Field::OptChat(v) => {
            out.write_bool(v.is_some());
            if let Some(v) = v {
              out.write_str(&v.to_json());
            }
          }
          Field::Item(_v) => {} // TODO: Slot data
          Field::Bool(v) => out.write_bool(*v),
          Field::Rotation(x, y, z) => {
            out.write_f32(*x);
            out.write_f32(*y);
            out.write_f32(*z);
          }
          Field::Position(v) => {
            out.write_i32(v.x());
            out.write_i32(v.y());
            out.write_i32(v.z());
          }
          Field::OptPosition(v) => {
            out.write_bool(v.is_some());
            if let Some(v) = v {
              out.write_i32(v.x());
              out.write_i32(v.y());
              out.write_i32(v.z());
            }
          }
          Field::Direction(v) => match v {
            Face::Down => out.write_varint(0),
            Face::Up => out.write_varint(1),
            Face::North => out.write_varint(2),
            Face::South => out.write_varint(3),
            Face::West => out.write_varint(4),
            Face::East => out.write_varint(5),
          },
          Field::OptUUID(v) => {
            out.write_bool(v.is_some());
            if let Some(v) = v {
              out.write_buf(&v.as_le_bytes());
            }
          }
          Field::BlockID(v) => out.write_varint(*v),
          Field::NBT(v) => out.write_buf(v),
          Field::Particle(v) => out.write_buf(v),
          Field::VillagerData(ty, p, l) => {
            out.write_varint(*ty);
            out.write_varint(*p);
            out.write_varint(*l);
          }
          Field::OptVarint(v) => out.write_varint(v.unwrap_or(0)),
          Field::Pose(v) => match v {
            Pose::Standing => out.write_varint(0),
            Pose::FallFlying => out.write_varint(1),
            Pose::Sleeping => out.write_varint(2),
            Pose::Swimming => out.write_varint(3),
            Pose::SpinAttack => out.write_varint(4),
            Pose::Sneaking => out.write_varint(5),
            Pose::Dying => out.write_varint(6),
          },
        }
      }
    }
    if self.ver == ProtocolVersion::V1_8 {
      out.write_varint(127);
    } else {
      out.write_u8(0xff);
    }
    out.into_inner()
  }
}
