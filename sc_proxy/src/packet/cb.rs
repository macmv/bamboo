use super::TypeConverter;
use sc_common::{
  gnet::cb::Packet as GPacket,
  net::{cb, cb::Packet},
  util::Buffer,
  version::ProtocolVersion,
};
use std::{error::Error, fmt};

#[derive(Debug, Clone)]
pub enum WriteError {
  InvalidVer,
}

impl fmt::Display for WriteError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::InvalidVer => write!(f, "invalid version"),
    }
  }
}

impl Error for WriteError {}

pub trait ToTcp {
  fn to_tcp(self, ver: ProtocolVersion, conv: &TypeConverter) -> Result<GPacket, WriteError>;
}

impl ToTcp for Packet {
  fn to_tcp(self, ver: ProtocolVersion, conv: &TypeConverter) -> Result<GPacket, WriteError> {
    Ok(match self {
      // Packet::Chunk { .. } => GPacket::ChunkDataV8 {},
      Packet::Chat { msg, ty } => {
        if ver < ProtocolVersion::V1_12_2 {
          GPacket::ChatV8 { chat_component: msg, ty: ty as i8 }
        } else {
          GPacket::ChatV12 { chat_component: msg, ty: None, unknown: vec![ty] }
        }
      }
      Packet::Chunk { pos, full, bit_map, sections } => {
        super::chunk(pos, full, bit_map, sections, ver, conv)
      }
      Packet::EntityLook { eid, yaw, pitch, on_ground } => GPacket::EntityLookV8 {
        entity_id: eid,
        pos_x: None,
        pos_y: None,
        pos_z: None,
        yaw,
        pitch,
        on_ground,
        field_149069_g: None,
      },
      Packet::EntityMove { eid, x, y, z, on_ground } => {
        if ver == ProtocolVersion::V1_8 {
          GPacket::EntityRelMoveV8 {
            entity_id: eid,
            pos_x: (x / (4096 / 32)) as i8,
            pos_y: (y / (4096 / 32)) as i8,
            pos_z: (z / (4096 / 32)) as i8,
            yaw: None,
            pitch: None,
            on_ground,
            field_149069_g: None,
          }
        } else {
          GPacket::EntityRelMoveV9 {
            entity_id: eid,
            pos_x: x.into(),
            pos_y: y.into(),
            pos_z: z.into(),
            yaw: None,
            pitch: None,
            on_ground,
            rotating: None,
          }
        }
      }
      Packet::EntityMoveLook { eid, x, y, z, yaw, pitch, on_ground } => {
        if ver == ProtocolVersion::V1_8 {
          GPacket::EntityLookMoveV8 {
            entity_id: eid,
            pos_x: (x / (4096 / 32)) as i8,
            pos_y: (y / (4096 / 32)) as i8,
            pos_z: (z / (4096 / 32)) as i8,
            yaw,
            pitch,
            on_ground,
            field_149069_g: None,
          }
        } else {
          GPacket::EntityLookMoveV9 {
            entity_id: eid,
            pos_x: x.into(),
            pos_y: y.into(),
            pos_z: z.into(),
            yaw,
            pitch,
            on_ground,
            rotating: None,
          }
        }
      }
      Packet::EntityPos { eid, x, y, z, yaw, pitch, on_ground } => {
        if ver == ProtocolVersion::V1_8 {
          GPacket::EntityTeleportV8 {
            entity_id: eid,
            pos_x: (x * 32.0) as i32,
            pos_y: (y * 32.0) as i32,
            pos_z: (z * 32.0) as i32,
            yaw,
            pitch,
            on_ground,
          }
        } else {
          GPacket::EntityTeleportV9 {
            entity_id: eid,
            pos_x: x,
            pos_y: y,
            pos_z: z,
            yaw,
            pitch,
            on_ground,
          }
        }
      }
      Packet::JoinGame {
        eid,
        hardcore_mode,
        game_mode,
        dimension,
        level_type,
        difficulty,
        view_distance,
        reduced_debug_info,
      } => {
        let mut out = Buffer::new(vec![]);
        out.write_u8(game_mode.id());
        if ver >= ProtocolVersion::V1_9_2 {
          out.write_i32(dimension.into());
        } else {
          out.write_i8(dimension.into());
        }
        out.write_i8(difficulty);
        if ver <= ProtocolVersion::V1_12_2 {
          // Max players. Ignored on the versions where its present.
          out.write_u8(0);
        }
        out.write_str(&level_type);
        if ver >= ProtocolVersion::V1_14_4 {
          out.write_varint(view_distance.into());
        }
        out.write_bool(reduced_debug_info);

        GPacket::JoinGameV8 {
          entity_id: eid,
          hardcore_mode,
          game_type: None,
          dimension: None,
          difficulty: None,
          max_players: None,
          world_type: None,
          reduced_debug_info: None,
          unknown: out.into_inner(),
        }
      }
      Packet::KeepAlive { id } => {
        if ver < ProtocolVersion::V1_12_2 {
          GPacket::KeepAliveV8 { id: id as i32 }
        } else {
          GPacket::KeepAliveV12 { id: id.into() }
        }
      }
      Packet::MultiBlockChange { pos, y, changes } => {
        super::multi_block_change(pos, y, changes, ver, conv)
      }
      Packet::PlayerHeader { header, footer } => GPacket::PlayerListHeaderV8 { header, footer },
      Packet::PlayerList { action } => {
        let id;
        let mut buf = Buffer::new(vec![]);
        match action {
          cb::PlayerListAction::Add(v) => {
            id = 0;
            buf.write_list(&v, |buf, v| {
              buf.write_uuid(v.id);
              buf.write_str(&v.name);
              buf.write_varint(0);
              buf.write_varint(v.game_mode.id().into());
              buf.write_varint(v.ping);
              buf.write_option(&v.display_name, |buf, v| buf.write_str(v));
            });
          }
          cb::PlayerListAction::UpdateGameMode(v) => {
            id = 1;
            buf.write_list(&v, |buf, v| {
              buf.write_uuid(v.id);
              buf.write_varint(v.game_mode.id().into());
            });
          }
          cb::PlayerListAction::UpdateLatency(v) => {
            id = 2;
            buf.write_list(&v, |buf, v| {
              buf.write_uuid(v.id);
              buf.write_varint(v.ping);
            });
          }
          cb::PlayerListAction::UpdateDisplayName(v) => {
            id = 3;
            buf.write_list(&v, |buf, v| {
              buf.write_uuid(v.id);
              buf.write_option(&v.display_name, |buf, v| buf.write_str(v));
            });
          }
          cb::PlayerListAction::Remove(v) => {
            id = 4;
            buf.write_list(&v, |buf, v| {
              buf.write_uuid(v.id);
            });
          }
        }
        GPacket::PlayerListV8 { action: id, players: None, unknown: buf.into_inner() }
      }
      Packet::SetPosLook { x, y, z, yaw, pitch, flags, teleport_id } => {
        let mut buf = Buffer::new(vec![]);
        buf.write_u8(flags);
        if ver >= ProtocolVersion::V1_9 {
          buf.write_varint(teleport_id as i32);
        }
        GPacket::PlayerPosLookV8 {
          x,
          y,
          z,
          yaw,
          pitch,
          field_179835_f: None,
          unknown: buf.into_inner(),
        }
      }
      Packet::SpawnPlayer { eid, id, x, y, z, yaw, pitch } => {
        if ver == ProtocolVersion::V1_8 {
          GPacket::SpawnPlayerV8 {
            entity_id: eid,
            player_id: id,
            x: (x * 32.0) as i32,
            y: (y * 32.0) as i32,
            z: (z * 32.0) as i32,
            yaw,
            pitch,
            current_item: 0,
            watcher: None,
            field_148958_j: None,
            // No entity metadata
            unknown: vec![0x7f],
          }
        } else if ver < ProtocolVersion::V1_15_2 {
          GPacket::SpawnPlayerV9 {
            entity_id: eid,
            unique_id: id,
            x,
            y,
            z,
            yaw,
            pitch,
            watcher: None,
            data_manager_entries: None,
            // No entity metadata
            unknown: vec![0xff],
          }
        } else {
          GPacket::SpawnPlayerV15 { id: eid, uuid: id, x, y, z, yaw, pitch }
        }
      }
      Packet::UnloadChunk { pos } => {
        if ver == ProtocolVersion::V1_8 {
          GPacket::ChunkDataV8 {
            chunk_x:        pos.x(),
            chunk_z:        pos.z(),
            field_149279_g: true,
            extracted_data: None,
            // Zero bit mask, then zero length varint
            unknown:        vec![0, 0, 0],
          }
        } else {
          GPacket::UnloadChunkV9 { x: pos.x(), z: pos.z() }
        }
      }
      Packet::UpdateViewPos { pos } => {
        if ver >= ProtocolVersion::V1_14 {
          GPacket::ChunkRenderDistanceCenterV14 { chunk_x: pos.x(), chunk_z: pos.z() }
        } else {
          panic!("cannot send UpdateViewPos for version {}", ver);
        }
      }
      _ => todo!("convert {:?} into generated packet", self),
    })
  }
}
