use super::{conv::entity::MetadataType, TypeConverter};
use bb_common::{
  metadata::{Field, Metadata, Pose},
  util::{Buffer, Face},
  version::ProtocolVersion,
};
use std::mem;

/// Serializes the entity metadata. This will not consume the metadata, and
/// will fail if there is invalid metadata fields given. This is for
/// cross-versioning reasons. Currently, this will panic when given bad data.
///
/// TODO: Return a `Result`.
pub fn metadata(ty: u32, meta: &Metadata, ver: ProtocolVersion, conv: &TypeConverter) -> Vec<u8> {
  let mut data = vec![];
  let mut out = Buffer::new(&mut data);
  for (&id, field) in &meta.fields {
    let (id, new_ty, old_ty) = conv.entity_metadata_types(ty, id, ver.block());

    debug_assert!(is_ty(&field, new_ty), "expected field to have type {new_ty:?}, got {field:?}");

    let mut field = field.clone();
    if !is_ty(&field, old_ty) {
      convert_field(&mut field, old_ty);
    }

    if ver == ProtocolVersion::V1_8 {
      // Index and type are the same byte in 1.8
      let mut index_type = id & 0x1f;
      match field {
        Field::Byte(_) | Field::Bool(_) => index_type |= 0 << 5,
        Field::Short(_) => index_type |= 1 << 5,
        Field::Int(_) => index_type |= 2 << 5,
        Field::Float(_) => index_type |= 3 << 5,
        Field::String(_) => index_type |= 4 << 5,
        Field::Item(_) => index_type |= 5 << 5,
        Field::Position(_) => index_type |= 6 << 5,
        Field::Rotation(_, _, _) => index_type |= 7 << 5,
        _ => unreachable!("cannot write {field:?} in 1.8"),
      }
      out.write_u8(index_type);
      match field {
        Field::Byte(v) => out.write_u8(v),
        Field::Bool(v) => out.write_bool(v),
        Field::Short(v) => out.write_i16(v),
        Field::Int(v) => out.write_i32(v),
        Field::Float(v) => out.write_f32(v),
        Field::String(v) => out.write_str(&v),
        Field::Item(item) => {
          let mut item = item.clone();
          conv.item(&mut item, ver.block());
          out.write_i16(item.id as i16);
          out.write_u8(item.count());
          out.write_i16(item.damage);
          out.write_u8(0x00); // TODO: NBT
        }
        Field::Position(v) => {
          out.write_i32(v.x());
          out.write_i32(v.y());
          out.write_i32(v.z());
        }
        Field::Rotation(x, y, z) => {
          out.write_f32(x);
          out.write_f32(y);
          out.write_f32(z);
        }
        _ => unreachable!(),
      }
    } else {
      out.write_varint(id.into());
      // Thank you minecraft. All of this is just for the metadata types.
      out.write_u8(match field {
        Field::Byte(_) => 0,
        Field::Varint(_) => 1,
        Field::Float(_) => 2,
        Field::String(_) => 3,
        Field::Chat(_) => 5,
        _ => {
          if ver >= ProtocolVersion::V1_13 {
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
                if ver >= ProtocolVersion::V1_14 {
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
                if ver == ProtocolVersion::V1_12 {
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
        Field::Byte(v) => out.write_u8(v),
        Field::Varint(v) => out.write_varint(v),
        Field::Float(v) => out.write_f32(v),
        Field::String(v) => out.write_str(&v),
        Field::Chat(v) => out.write_str(&v),
        Field::OptChat(v) => {
          out.write_bool(v.is_some());
          if let Some(v) = v {
            out.write_str(&v);
          }
        }
        Field::Item(item) => {
          let mut item = item.clone();
          if ver < ProtocolVersion::V1_13 {
            if item.count() == 0 {
              out.write_i16(-1);
            } else {
              conv.item(&mut item, ver.block());
              out.write_i16(item.id as i16);
              if item.id() != -1 {
                out.write_u8(item.count());
                out.write_i16(item.damage);
                out.write_u8(0); // TODO: Write nbt data
              }
            }
          } else {
            let present = item.count() != 0 && item.id() != -1;
            conv.item(&mut item, ver.block());
            out.write_bool(present);
            if present {
              out.write_varint(item.id as i32);
              out.write_u8(item.count());
              out.write_u8(0x00); // TODO: Write nbt data
            }
          }
        }
        Field::Bool(v) => out.write_bool(v),
        Field::Rotation(x, y, z) => {
          out.write_f32(x);
          out.write_f32(y);
          out.write_f32(z);
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
          Face::Bottom => out.write_varint(0),
          Face::Top => out.write_varint(1),
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
        Field::BlockID(v) => out.write_varint(v),
        Field::NBT(v) => out.write_buf(&v),
        Field::Particle(v) => out.write_buf(&v),
        Field::VillagerData(ty, p, l) => {
          out.write_varint(ty);
          out.write_varint(p);
          out.write_varint(l);
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
  if ver == ProtocolVersion::V1_8 {
    out.write_varint(127);
  } else {
    out.write_u8(0xff);
  }
  data
}

fn is_ty(field: &Field, ty: MetadataType) -> bool {
  match field {
    // Only valid on 1.8
    Field::Short(_) => matches!(ty, MetadataType::Byte),
    Field::Int(_) => matches!(ty, MetadataType::Int),

    Field::Byte(_) => matches!(ty, MetadataType::Byte),
    Field::Float(_) => matches!(ty, MetadataType::Float),
    Field::String(_) => matches!(ty, MetadataType::String),
    Field::Item(_) => matches!(ty, MetadataType::Item),
    Field::Position(_) => matches!(ty, MetadataType::Position),
    Field::Rotation(..) => matches!(ty, MetadataType::Rotation),

    Field::Varint(_) => matches!(ty, MetadataType::VarInt),
    Field::Chat(_) => matches!(ty, MetadataType::Chat),
    Field::Bool(_) => matches!(ty, MetadataType::Bool),
    Field::OptPosition(_) => matches!(ty, MetadataType::OptPosition),
    Field::Direction(_) => matches!(ty, MetadataType::Direction),
    Field::OptUUID(_) => matches!(ty, MetadataType::OptUUID),
    Field::BlockID(_) => matches!(ty, MetadataType::BlockID),

    Field::NBT(_) => matches!(ty, MetadataType::NBT),

    Field::OptChat(_) => matches!(ty, MetadataType::OptChat),
    Field::Particle(_) => matches!(ty, MetadataType::Particle),

    Field::VillagerData(..) => matches!(ty, MetadataType::VillagerData),
    Field::OptVarint(_) => matches!(ty, MetadataType::OptVarInt),
    Field::Pose(_) => matches!(ty, MetadataType::Pose),
  }
}
fn convert_field(field: &mut Field, ty: MetadataType) {
  // Replace `field` with a temporary, so that we can move out of the old data.
  let old_field = mem::replace(field, Field::Bool(false));
  match (old_field, ty) {
    (Field::OptChat(msg), MetadataType::String) => {
      *field = Field::String(msg.unwrap_or_else(String::new))
    }
    (field, ty) => panic!("cannot convert {field:?} into {ty:?}"),
  }
}
