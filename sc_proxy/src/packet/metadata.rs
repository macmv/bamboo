use super::TypeConverter;
use sc_common::{
  metadata::{Field, Metadata},
  util::Buffer,
  version::ProtocolVersion,
};

/// Serializes the entity metadata. This will not consume the metadata, and
/// will never fail. The `set` functions are where error handling is done.
pub fn metadata(meta: &Metadata, ver: ProtocolVersion, conv: &TypeConverter) -> Vec<u8> {
  let mut data = vec![];
  let mut out = Buffer::new(&mut data);
  for (id, field) in &meta.fields {
    id = conv.entity_metadata_to_old(id, ver);
    if ver == ProtocolVersion::V1_8 {
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
          let (it, damage) = conv.item_to_old(v.id() as u32, ver.block());
          out.write_i16(it as i16);
          out.write_u8(v.count());
          out.write_i16(damage as i16);
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
        Field::Byte(v) => out.write_u8(*v),
        Field::Varint(v) => out.write_varint(*v),
        Field::Float(v) => out.write_f32(*v),
        Field::String(v) => out.write_str(v),
        Field::Chat(v) => out.write_str(v),
        Field::OptChat(v) => {
          out.write_bool(v.is_some());
          if let Some(v) = v {
            out.write_str(&v);
          }
        }
        Field::Item(_v) => todo!("items on new versions in entity metadata"),
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
  if ver == ProtocolVersion::V1_8 {
    out.write_varint(127);
  } else {
    out.write_u8(0xff);
  }
  data
}
