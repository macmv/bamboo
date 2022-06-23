use super::metadata;
use crate::{
  gnet::{cb::Packet as GPacket, tcp},
  stream::PacketStream,
  Conn,
};
use bb_common::{
  nbt,
  net::{
    cb,
    cb::{
      Animation, ArmorSlot, CommandType, EquipmentSlot, ObjectiveAction, ObjectiveType, Packet,
      ScoreboardAction, ScoreboardDisplay, SoundCategory, TeamAction, TeamInfo, TeamRule,
      TitleAction,
    },
  },
  util::{chat, Buffer, GameMode, Hand, UUID},
  version::ProtocolVersion,
};
use serde::Serialize;
use smallvec::SmallVec;
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
  fn to_tcp<S: PacketStream + Send + Sync>(
    self,
    conn: &mut Conn<S>,
  ) -> Result<SmallVec<[GPacket; 2]>, WriteError>;
}

impl ToTcp for Packet {
  fn to_tcp<S: PacketStream + Send + Sync>(
    self,
    conn: &mut Conn<S>,
  ) -> Result<SmallVec<[GPacket; 2]>, WriteError> {
    let ver = conn.ver();

    Ok(smallvec![match self {
      Packet::Abilities {
        invulnerable,
        flying,
        allow_flying,
        insta_break,
        fly_speed,
        walk_speed,
      } =>
        if ver < ProtocolVersion::V1_16_5 {
          GPacket::PlayerAbilitiesV8 {
            invulnerable,
            flying,
            allow_flying,
            creative_mode: insta_break,
            fly_speed: fly_speed * 0.05,
            walk_speed: walk_speed * 0.1,
            v_2: 0,
          }
        } else {
          GPacket::PlayerAbilitiesV16 {
            invulnerable,
            flying,
            allow_flying,
            creative_mode: insta_break,
            fly_speed: fly_speed * 0.05,
            walk_speed: walk_speed * 0.1,
            v_2: 0,
          }
        },
      Packet::Animation { eid, kind } => {
        if ver == ProtocolVersion::V1_8 {
          GPacket::AnimationV8 {
            entity_id: eid,
            ty:        match kind {
              Animation::Swing(_) => 0,
              Animation::Damage => 1,
              Animation::LeaveBed => 2,
              Animation::Crit => 4,
              Animation::MagicCrit => 5,
            },
          }
        } else {
          GPacket::AnimationV8 {
            entity_id: eid,
            ty:        match kind {
              Animation::Swing(Hand::Main) => 0,
              Animation::Damage => 1,
              Animation::LeaveBed => 2,
              Animation::Swing(Hand::Off) => 0,
              Animation::Crit => 4,
              Animation::MagicCrit => 5,
            },
          }
        }
      }
      Packet::BlockUpdate { pos, state } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        buf.write_varint(state as i32);
        GPacket::BlockUpdateV8 { block_position: pos, unknown: data }
      }
      Packet::ChangeGameState { action } => {
        use bb_common::net::cb::ChangeGameState as Action;
        let reason = match action {
          Action::InvalidBed => 0,
          Action::EndRaining => 1,
          Action::BeginRaining => 2,
          Action::GameMode(_) => 3,
          Action::EnterCredits => 4,
          Action::DemoMessage(_) => 5,
          Action::ArrowHitPlayer => 6,
          Action::FadeValue(_) => 7,
          Action::FadeTime(_) => 8,
          Action::PufferfishSting => {
            if ver < ProtocolVersion::V1_14_4 {
              return Err(WriteError::InvalidVer);
            } else {
              9
            }
          }
          Action::ElderGuardianAppear => 10,
          Action::EnableRespawnScreen(_) => {
            if ver < ProtocolVersion::V1_15_2 {
              return Err(WriteError::InvalidVer);
            } else {
              9
            }
          }
        };
        let value = match action {
          Action::GameMode(mode) => match mode {
            GameMode::Survival => 0.0,
            GameMode::Creative => 1.0,
            GameMode::Adventure => 2.0,
            GameMode::Spectator => 3.0,
          },
          Action::DemoMessage(v) => v,
          Action::FadeValue(v) => v,
          Action::FadeTime(v) => v,
          Action::EnableRespawnScreen(enable) => {
            if enable {
              0.0
            } else {
              1.0
            }
          }
          _ => 0.0,
        };
        if ver >= ProtocolVersion::V1_16_5 {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_u8(reason);
          buf.write_f32(value);
          GPacket::ChangeGameStateV16 { unknown: data }
        } else {
          GPacket::ChangeGameStateV8 { state: reason.into(), field_149141_c: value }
        }
      }
      Packet::Chat { msg, ty } => {
        if ver >= ProtocolVersion::V1_19 {
          GPacket::SystemChatV19 {
            a: msg.to_json(), // content
            b: 1,             // type
          }
        } else if ver >= ProtocolVersion::V1_16_5 {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_u8(ty);
          buf.write_uuid(UUID::from_u128(0));
          GPacket::ChatV12 { chat_component: msg.to_json(), unknown: data }
        } else if ver >= ProtocolVersion::V1_12_2 {
          GPacket::ChatV12 { chat_component: msg.to_json(), unknown: vec![ty] }
        } else {
          GPacket::ChatV8 { chat_component: msg.to_json(), ty: ty as i8 }
        }
      }
      Packet::Chunk { pos, full, sections, sky_light, block_light } => {
        return Ok(super::chunk(
          super::ChunkWithPos { pos, full, sections, sky_light, block_light },
          ver,
          conn.conv(),
        ));
      }
      Packet::CommandList { nodes, root } => {
        if ver < ProtocolVersion::V1_13 {
          panic!("command tree doesn't exist for version {}", ver);
        }
        if ver >= ProtocolVersion::V1_19 {
          return Ok(smallvec![]);
        }
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        buf.write_list(&nodes, |buf, node| {
          let mut flags = match node.ty {
            CommandType::Root => 0,
            CommandType::Literal => 1,
            CommandType::Argument => 2,
          };
          if node.executable {
            flags |= 0x04;
          }
          if node.redirect.is_some() {
            flags |= 0x08;
          }
          if node.suggestion.is_some() {
            flags |= 0x10;
          }
          buf.write_u8(flags);
          buf.write_list(&node.children, |buf, child| buf.write_varint(*child as i32));
          if let Some(redirect) = node.redirect {
            buf.write_varint(redirect as i32);
          }
          if node.ty == CommandType::Literal || node.ty == CommandType::Argument {
            buf.write_str(&node.name);
          }
          if node.ty == CommandType::Argument {
            if ver >= ProtocolVersion::V1_19 {
              // buf.write_varint(conn.conv().command_to_old(node.parser.id(),
              // ver));
            } else {
              buf.write_str(&node.parser);
            }
            buf.write_buf(&node.properties);
          }
          if let Some(suggestion) = &node.suggestion {
            buf.write_str(suggestion);
          }
        });
        buf.write_varint(root as i32);
        if ver >= ProtocolVersion::V1_19 {
          GPacket::CommandTreeV19 { unknown: data }
        } else if ver >= ProtocolVersion::V1_16_5 {
          GPacket::CommandTreeV16 { unknown: data }
        } else {
          GPacket::CommandTreeV14 { unknown: data }
        }
      }
      Packet::CollectItem { item_eid, player_eid, amount } => {
        if ver >= ProtocolVersion::V1_11_2 {
          GPacket::CollectItemV11 {
            collected_item_entity_id: item_eid,
            entity_id:                player_eid,
            field_191209_c:           amount.into(),
          }
        } else {
          GPacket::CollectItemV8 {
            collected_item_entity_id: item_eid,
            entity_id:                player_eid,
          }
        }
      }
      Packet::EntityEquipment { eid, slot, mut item } => {
        conn.conv().item(&mut item, ver.block());
        if ver >= ProtocolVersion::V1_16_5 {
          let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
          // TODO: Multiple equipment updates can be sent in one packet on this version.
          // This is serialized as an array, where this byte has the top bit set if there
          // is another entry. We keep this top bit unset, as this is the single (and
          // last) entry.
          buf.write_u8(match slot {
            EquipmentSlot::Hand(Hand::Main) => 0,
            EquipmentSlot::Hand(Hand::Off) => 1,
            EquipmentSlot::Armor(ArmorSlot::Boots) => 2,
            EquipmentSlot::Armor(ArmorSlot::Leggings) => 3,
            EquipmentSlot::Armor(ArmorSlot::Chestplate) => 4,
            EquipmentSlot::Armor(ArmorSlot::Helmet) => 5,
          });
          buf.write_item(&item);
          GPacket::EntityEquipmentV16 { id: eid, unknown: buf.serialize() }
        } else if ver >= ProtocolVersion::V1_9_4 {
          let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
          buf.write_item(&item);
          GPacket::EntityEquipmentV9 {
            entity_id:      eid,
            equipment_slot: match slot {
              EquipmentSlot::Hand(Hand::Main) => 0,
              EquipmentSlot::Hand(Hand::Off) => 1,
              EquipmentSlot::Armor(ArmorSlot::Boots) => 2,
              EquipmentSlot::Armor(ArmorSlot::Leggings) => 3,
              EquipmentSlot::Armor(ArmorSlot::Chestplate) => 4,
              EquipmentSlot::Armor(ArmorSlot::Helmet) => 5,
            },
            unknown:        buf.serialize(),
          }
        } else {
          let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
          buf.write_item(&item);
          GPacket::EntityEquipmentV8 {
            entity_id:      eid,
            equipment_slot: match slot {
              EquipmentSlot::Hand(Hand::Main) => 0,
              // 1.8 client can't see offhand, so we can't really send them anything
              EquipmentSlot::Hand(Hand::Off) => return Ok(smallvec![]),
              EquipmentSlot::Armor(ArmorSlot::Boots) => 1,
              EquipmentSlot::Armor(ArmorSlot::Leggings) => 2,
              EquipmentSlot::Armor(ArmorSlot::Chestplate) => 3,
              EquipmentSlot::Armor(ArmorSlot::Helmet) => 4,
            },
            unknown:        buf.serialize(),
          }
        }
      }
      Packet::EntityHeadLook { eid, yaw } => {
        GPacket::EntityHeadLookV8 { entity_id: eid, yaw }
      }
      Packet::EntityLook { eid, yaw, pitch, on_ground } => {
        if ver >= ProtocolVersion::V1_17_1 {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_varint(eid);
          buf.write_i8(yaw);
          buf.write_i8(pitch);
          buf.write_bool(on_ground);
          GPacket::EntityLookV17 { unknown: data, v_1: 0, v_2: 0, v_3: 0, v_4: 0 }
        } else {
          GPacket::EntityLookV8 { entity_id: eid, yaw, pitch, on_ground }
        }
      }
      Packet::EntityMove { eid, x, y, z, on_ground } => {
        if ver >= ProtocolVersion::V1_17_1 {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_varint(eid);
          buf.write_i16(x);
          buf.write_i16(y);
          buf.write_i16(z);
          buf.write_bool(on_ground);
          GPacket::EntityRelMoveV17 {
            unknown: data,
            v_1:     0,
            v_2:     0,
            v_3:     0,
            v_4:     0,
            v_5:     0,
          }
        } else if ver >= ProtocolVersion::V1_9_4 {
          GPacket::EntityRelMoveV9 {
            entity_id: eid,
            pos_x: x.into(),
            pos_y: y.into(),
            pos_z: z.into(),
            on_ground,
          }
        } else {
          GPacket::EntityRelMoveV8 {
            entity_id: eid,
            pos_x: (x / (4096 / 32)) as i8,
            pos_y: (y / (4096 / 32)) as i8,
            pos_z: (z / (4096 / 32)) as i8,
            on_ground,
          }
        }
      }
      Packet::EntityMoveLook { eid, x, y, z, yaw, pitch, on_ground } => {
        if ver >= ProtocolVersion::V1_17_1 {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_varint(eid);
          buf.write_i16(x);
          buf.write_i16(y);
          buf.write_i16(z);
          buf.write_i8(yaw);
          buf.write_i8(pitch);
          buf.write_bool(on_ground);
          GPacket::EntityLookMoveV17 {
            unknown: data,
            v_1:     0,
            v_2:     0,
            v_3:     0,
            v_4:     0,
            v_5:     0,
            v_6:     0,
            v_7:     0,
          }
        } else if ver >= ProtocolVersion::V1_9_4 {
          GPacket::EntityLookMoveV9 {
            entity_id: eid,
            pos_x: x.into(),
            pos_y: y.into(),
            pos_z: z.into(),
            yaw,
            pitch,
            on_ground,
          }
        } else {
          GPacket::EntityLookMoveV8 {
            entity_id: eid,
            pos_x: (x / (4096 / 32)) as i8,
            pos_y: (y / (4096 / 32)) as i8,
            pos_z: (z / (4096 / 32)) as i8,
            yaw,
            pitch,
            on_ground,
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
      Packet::EntityStatus { eid, status } => {
        GPacket::EntityStatusV8 { entity_id: eid, logic_opcode: status as i8 }
      }
      Packet::EntityMetadata { eid, ty, meta } => {
        GPacket::EntityMetadataV8 {
          entity_id: eid,
          unknown:   match metadata(ty, &meta, ver, conn.conv()) {
            Some(m) => m,
            None => return Ok(smallvec![]),
          },
        }
      }
      Packet::EntityVelocity { eid, x, y, z } => {
        GPacket::EntityVelocityV8 {
          entity_id: eid,
          motion_x:  x.into(),
          motion_y:  y.into(),
          motion_z:  z.into(),
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
        enable_respawn_screen,
        world_height,
        world_min_y,
      } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        if ver >= ProtocolVersion::V1_16_5 {
          buf.write_u8(game_mode.id());
          buf.write_i8(-1); // no previous_game_mode

          // List of worlds
          buf.write_varint(1);
          buf.write_str("minecraft:overworld");

          write_dimensions(&mut buf, ver, world_height, world_min_y);

          // Hashed world seed, used for biomes client side.
          buf.write_u64(0);
          // Max players (ignored)
          buf.write_varint(0);

          buf.write_varint(view_distance.into());
          if ver >= ProtocolVersion::V1_18 {
            // The simulation distance
            buf.write_varint(view_distance.into());
          }
          buf.write_bool(reduced_debug_info);
          buf.write_bool(enable_respawn_screen);
          buf.write_bool(false); // Is debug; cannot be modified, has preset blocks
          buf.write_bool(false); // Is flat; changes fog
          if ver >= ProtocolVersion::V1_19 {
            // Last death location.
            buf.write_option(&None, |_, _: &()| {});
          }
        } else if ver >= ProtocolVersion::V1_15_2 {
          buf.write_i32(dimension.into());
          // Hashed world seed, used for biomes
          buf.write_u64(0);
          // Max players (ignored)
          buf.write_u8(0);
          // World type
          buf.write_str("default");
          buf.write_varint(view_distance.into());
          buf.write_bool(reduced_debug_info);
          buf.write_bool(enable_respawn_screen);
        } else if ver >= ProtocolVersion::V1_14_4 {
          buf.write_i32(dimension.into());
          // Max players (ignored)
          buf.write_u8(0);
          // World type
          buf.write_str("default");
          buf.write_varint(view_distance.into());
          buf.write_bool(reduced_debug_info);
        } else {
          buf.write_bool(reduced_debug_info);
        }

        match ver.maj().unwrap() {
          8 => GPacket::JoinGameV8 {
            entity_id: eid,
            hardcore_mode,
            game_type: game_mode.id(),
            dimension: dimension.into(),
            difficulty: difficulty.into(),
            max_players: 0,
            world_type: level_type,
            unknown: data,
          },
          9..=13 => GPacket::JoinGameV9 {
            player_id: eid,
            hardcore_mode,
            game_type: game_mode.id(),
            dimension: dimension.into(),
            difficulty: difficulty.into(),
            max_players: 0,
            world_type: level_type,
            unknown: data,
          },
          14..=15 => GPacket::JoinGameV14 {
            player_entity_id: eid,
            hardcore:         hardcore_mode,
            unknown:          data,
            v_2:              0,
          },
          16 => GPacket::JoinGameV16 {
            player_entity_id: eid,
            hardcore:         hardcore_mode,
            unknown:          data,
          },
          17 | 18 | 19 => GPacket::JoinGameV17 {
            player_entity_id: eid,
            hardcore:         hardcore_mode,
            unknown:          data,
          },
          _ => unimplemented!(),
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
        super::multi_block_change(pos, y, changes, ver, conn.conv())
      }
      Packet::Particle { id, long, pos, offset, data_float, count, data: particle_data } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        let old_id = match conn.conv().particle_to_old(id as u32, ver.block()) {
          Some(id) => id as i32,
          None => return Ok(smallvec![]),
        };
        if ver >= ProtocolVersion::V1_14_4 {
          buf.write_i32(old_id);
          buf.write_bool(long);
          buf.write_f64(pos.x());
          buf.write_f64(pos.y());
          buf.write_f64(pos.z());
          buf.write_f32(offset.x() as f32);
          buf.write_f32(offset.y() as f32);
          buf.write_f32(offset.z() as f32);
          buf.write_f32(data_float);
          buf.write_i32(count);
          buf.write_buf(&particle_data);
          GPacket::ParticleV14 { unknown: data }
        } else {
          buf.write_i32(old_id);
          buf.write_bool(long);
          buf.write_f32(pos.x() as f32);
          buf.write_f32(pos.y() as f32);
          buf.write_f32(pos.z() as f32);
          buf.write_f32(offset.x() as f32);
          buf.write_f32(offset.y() as f32);
          buf.write_f32(offset.z() as f32);
          buf.write_f32(data_float);
          buf.write_i32(count);
          buf.write_buf(&particle_data);
          GPacket::ParticleV8 { unknown: data }
        }
      }
      Packet::PlayerHeader { header, footer } => {
        GPacket::PlayerListHeaderV8 { header, footer }
      }
      Packet::PlayerList { action } => {
        let id;
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
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
              // The user's public key
              if ver >= ProtocolVersion::V1_19 {
                buf.write_option(&None, |_, _: &()| {});
              }
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
              buf.write_option(&v.display_name, |buf, v| buf.write_str(&v.to_json()));
            });
          }
          cb::PlayerListAction::Remove(v) => {
            id = 4;
            buf.write_list(&v, |buf, v| {
              buf.write_uuid(v.id);
            });
          }
        }
        if ver < ProtocolVersion::V1_17_1 {
          GPacket::PlayerListV8 { action: id, unknown: data, v_2: 0 }
        } else {
          GPacket::PlayerListV17 { action: id, unknown: data }
        }
      }
      Packet::PlaySound { name, category, pos, volume, pitch } => {
        if ver >= ProtocolVersion::V1_14_4 {
          GPacket::CustomSoundV14 {
            id: name,
            category: match category {
              SoundCategory::Master => 0,
              SoundCategory::Music => 1,
              SoundCategory::Records => 2,
              SoundCategory::Weather => 3,
              SoundCategory::Blocks => 4,
              SoundCategory::Hostile => 5,
              SoundCategory::Neutral => 6,
              SoundCategory::Players => 7,
              SoundCategory::Ambient => 8,
              SoundCategory::Voice => 9,
            },
            fixed_x: (pos.x() * 8.0) as i32,
            fixed_y: (pos.y() * 8.0) as i32,
            fixed_z: (pos.z() * 8.0) as i32,
            volume,
            pitch,
          }
        } else if ver >= ProtocolVersion::V1_10_2 {
          GPacket::CustomSoundV10 {
            sound_name: name,
            category: match category {
              SoundCategory::Master => 0,
              SoundCategory::Music => 1,
              SoundCategory::Records => 2,
              SoundCategory::Weather => 3,
              SoundCategory::Blocks => 4,
              SoundCategory::Hostile => 5,
              SoundCategory::Neutral => 6,
              SoundCategory::Players => 7,
              SoundCategory::Ambient => 8,
              SoundCategory::Voice => 9,
            },
            x: (pos.x() * 8.0) as i32,
            y: (pos.y() * 8.0) as i32,
            z: (pos.z() * 8.0) as i32,
            volume,
            pitch,
          }
        } else if ver >= ProtocolVersion::V1_9_4 {
          GPacket::CustomSoundV9 {
            sound_name: name,
            category: match category {
              SoundCategory::Master => 0,
              SoundCategory::Music => 1,
              SoundCategory::Records => 2,
              SoundCategory::Weather => 3,
              SoundCategory::Blocks => 4,
              SoundCategory::Hostile => 5,
              SoundCategory::Neutral => 6,
              SoundCategory::Players => 7,
              SoundCategory::Ambient => 8,
              SoundCategory::Voice => 9,
            },
            x: (pos.x() * 8.0) as i32,
            y: (pos.y() * 8.0) as i32,
            z: (pos.z() * 8.0) as i32,
            volume,
            pitch: (pitch * 128.0) as i32,
          }
        } else {
          GPacket::PlaySoundV8 {
            sound_name:   name,
            pos_x:        (pos.x() * 8.0) as i32,
            pos_y:        (pos.y() * 8.0) as i32,
            pos_z:        (pos.z() * 8.0) as i32,
            sound_volume: volume,
            sound_pitch:  (pitch * 128.0) as i32,
          }
        }
      }
      Packet::PluginMessage { channel, data } => {
        // No length prefix for data, it is inferred from packet length.
        if ver < ProtocolVersion::V1_14_4 {
          GPacket::CustomPayloadV8 { channel, unknown: data, v_2: 0 }
        } else {
          GPacket::CustomPayloadV14 { channel, unknown: data, v_2: 0 }
        }
      }
      Packet::RemoveEntities { eids } => {
        if ver >= ProtocolVersion::V1_17_1 {
          GPacket::DestroyEntitiesV17 { entity_ids: eids }
        } else {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_list(&eids, |buf, &e| buf.write_varint(e));
          GPacket::DestroyEntitiesV8 { unknown: data }
        }
      }
      Packet::ScoreboardDisplay { position, objective } => {
        let pos = match position {
          ScoreboardDisplay::List => 0,
          ScoreboardDisplay::Sidebar => 1,
          ScoreboardDisplay::BelowName => 2,
        };
        if ver < ProtocolVersion::V1_18 {
          GPacket::ScoreboardDisplayV8 { position: pos, score_name: objective }
        } else {
          GPacket::ScoreboardDisplayV18 { slot: pos, name: objective }
        }
      }
      Packet::ScoreboardObjective { mode, objective } => {
        let m = match mode {
          ObjectiveAction::Create { .. } => 0,
          ObjectiveAction::Remove => 1,
          ObjectiveAction::Update { .. } => 2,
        };
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        match mode {
          ObjectiveAction::Create { value, ty } | ObjectiveAction::Update { value, ty } => {
            if ver <= ProtocolVersion::V1_12_2 {
              buf.write_str(&value.to_codes());
            } else {
              buf.write_str(&value.to_json());
            }
            buf.write_varint(match ty {
              ObjectiveType::Integer => 0,
              ObjectiveType::Hearts => 1,
            });
          }
          _ => {}
        }
        if ver < ProtocolVersion::V1_18 {
          GPacket::ScoreboardObjectiveV8 {
            objective_name: objective,
            field_149342_c: m,
            unknown:        data,
          }
        } else {
          GPacket::ScoreboardObjectiveV18 { name: objective, mode: m, unknown: data }
        }
      }
      Packet::ScoreboardUpdate { username, objective, action } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        if ver >= ProtocolVersion::V1_18 {
          buf.write_str(&objective);
          match action {
            ScoreboardAction::Create(score) => buf.write_varint(score),
            ScoreboardAction::Remove => {}
          }
          GPacket::UpdateScoreV18 {
            player_name: username,
            mode:        match action {
              ScoreboardAction::Create(_) => 0,
              ScoreboardAction::Remove => 1,
            },
            unknown:     data,
            v_2:         "".into(),
          }
        } else if ver >= ProtocolVersion::V1_14_4 {
          buf.write_str(&objective);
          match action {
            ScoreboardAction::Create(score) => buf.write_varint(score),
            ScoreboardAction::Remove => {}
          }
          GPacket::UpdateScoreV14 {
            player_name: username,
            mode:        match action {
              ScoreboardAction::Create(_) => 0,
              ScoreboardAction::Remove => 1,
            },
            unknown:     data,
            v_2:         "".into(),
          }
        } else {
          match action {
            ScoreboardAction::Create(score) => buf.write_varint(score),
            ScoreboardAction::Remove => {}
          }
          GPacket::UpdateScoreV8 {
            name: username,
            objective,
            action: match action {
              ScoreboardAction::Create(_) => 0,
              ScoreboardAction::Remove => 1,
            },
            unknown: data,
          }
        }
      }
      Packet::SetPosLook { pos, yaw, pitch, flags, teleport_id, should_dismount } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        buf.write_u8(flags);
        if ver >= ProtocolVersion::V1_9 {
          buf.write_varint(teleport_id as i32);
        }
        if ver >= ProtocolVersion::V1_17_1 {
          buf.write_bool(should_dismount);
        }
        GPacket::PlayerPosLookV8 { x: pos.x(), y: pos.y(), z: pos.z(), yaw, pitch, unknown: data }
      }
      Packet::SpawnLivingEntity {
        eid,
        id,
        ty,
        pos,
        yaw,
        pitch,
        head_yaw,
        vel_x,
        vel_y,
        vel_z,
        meta,
      } => {
        let new_ty = ty;
        let ty = conn.conv().entity_to_old(ty, ver.block()) as i32;
        if ver >= ProtocolVersion::V1_15_2 {
          let spawn = GPacket::SpawnMobV15 {
            id: eid,
            uuid: id,
            entity_type_id: ty,
            x: pos.x(),
            y: pos.y(),
            z: pos.z(),
            velocity_x: vel_x.into(),
            velocity_y: vel_y.into(),
            velocity_z: vel_z.into(),
            yaw,
            pitch,
            head_yaw,
          };
          if !meta.fields.is_empty() {
            match metadata(new_ty, &meta, ver, conn.conv()) {
              Some(data) => {
                return Ok(smallvec![
                  spawn,
                  GPacket::EntityMetadataV8 { entity_id: eid, unknown: data }
                ])
              }
              None => spawn,
            }
          } else {
            spawn
          }
        } else if ver >= ProtocolVersion::V1_11 {
          GPacket::SpawnMobV11 {
            entity_id: eid,
            unique_id: id,
            ty,
            x: pos.x(),
            y: pos.y(),
            z: pos.z(),
            velocity_x: vel_x.into(),
            velocity_y: vel_y.into(),
            velocity_z: vel_z.into(),
            yaw,
            pitch,
            head_pitch: head_yaw,
            unknown: match metadata(new_ty, &meta, ver, conn.conv()) {
              Some(m) => m,
              None => return Ok(smallvec![]),
            },
          }
        } else if ver >= ProtocolVersion::V1_9 {
          GPacket::SpawnMobV9 {
            entity_id: eid,
            unique_id: id,
            ty,
            x: pos.x(),
            y: pos.y(),
            z: pos.z(),
            velocity_x: vel_x.into(),
            velocity_y: vel_y.into(),
            velocity_z: vel_z.into(),
            yaw,
            pitch,
            head_pitch: head_yaw,
            unknown: match metadata(new_ty, &meta, ver, conn.conv()) {
              Some(m) => m,
              None => return Ok(smallvec![]),
            },
          }
        } else {
          GPacket::SpawnMobV8 {
            entity_id: eid,
            ty,
            x: (pos.x() * 32.0) as i32,
            y: (pos.y() * 32.0) as i32,
            z: (pos.z() * 32.0) as i32,
            velocity_x: vel_x.into(),
            velocity_y: vel_y.into(),
            velocity_z: vel_z.into(),
            yaw,
            pitch,
            head_pitch: head_yaw,
            unknown: match metadata(new_ty, &meta, ver, conn.conv()) {
              Some(m) => m,
              None => return Ok(smallvec![]),
            },
          }
        }
      }
      Packet::SpawnEntity {
        eid,
        id,
        ty,
        pos,
        yaw,
        pitch,
        vel_x,
        vel_y,
        vel_z,
        meta,
        data: data_int,
      } => {
        let new_ty = ty;
        let ty = conn.conv().entity_to_old(ty, ver.block()) as i32;
        let spawn = if ver >= ProtocolVersion::V1_14_4 {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_varint(ty);
          buf.write_f64(pos.x());
          buf.write_f64(pos.y());
          buf.write_f64(pos.z());
          buf.write_i8(pitch);
          buf.write_i8(yaw);
          buf.write_i32(data_int); // data
          buf.write_i16(vel_x);
          buf.write_i16(vel_y);
          buf.write_i16(vel_z);
          GPacket::SpawnObjectV14 { id: eid, uuid: id, unknown: data }
        } else if ver >= ProtocolVersion::V1_9 {
          GPacket::SpawnObjectV9 {
            entity_id: eid,
            unique_id: id,
            ty:        object_ty(ty),
            x:         pos.x(),
            y:         pos.y(),
            z:         pos.z(),
            yaw:       yaw.into(),
            pitch:     pitch.into(),
            speed_x:   vel_x.into(),
            speed_y:   vel_y.into(),
            speed_z:   vel_z.into(),
            data:      data_int,
          }
        } else {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_i16(vel_x);
          buf.write_i16(vel_y);
          buf.write_i16(vel_z);
          GPacket::SpawnObjectV8 {
            entity_id:      eid,
            ty:             object_ty(ty),
            x:              (pos.x() * 32.0) as i32,
            y:              (pos.y() * 32.0) as i32,
            z:              (pos.z() * 32.0) as i32,
            yaw:            yaw.into(),
            pitch:          pitch.into(),
            field_149020_k: data_int,
            unknown:        data,
          }
        };
        if !meta.fields.is_empty() {
          match metadata(new_ty, &meta, ver, conn.conv()) {
            Some(data) => {
              return Ok(smallvec![
                spawn,
                GPacket::EntityMetadataV8 { entity_id: eid, unknown: data }
              ])
            }
            None => spawn,
          }
        } else {
          spawn
        }
      }
      Packet::SpawnPlayer { eid, id, ty, pos, yaw, pitch, meta } => {
        if ver == ProtocolVersion::V1_8 {
          GPacket::SpawnPlayerV8 {
            entity_id: eid,
            player_id: id,
            x: (pos.x() * 32.0) as i32,
            y: (pos.y() * 32.0) as i32,
            z: (pos.z() * 32.0) as i32,
            yaw,
            pitch,
            current_item: 0,
            unknown: match metadata(ty, &meta, ver, conn.conv()) {
              Some(m) => m,
              None => return Ok(smallvec![]),
            },
          }
        } else if ver < ProtocolVersion::V1_15_2 {
          GPacket::SpawnPlayerV9 {
            entity_id: eid,
            unique_id: id,
            x: pos.x(),
            y: pos.y(),
            z: pos.z(),
            yaw,
            pitch,
            unknown: match metadata(ty, &meta, ver, conn.conv()) {
              Some(m) => m,
              None => return Ok(smallvec![]),
            },
          }
        } else {
          let spawn = GPacket::SpawnPlayerV15 {
            id: eid,
            uuid: id,
            x: pos.x(),
            y: pos.y(),
            z: pos.z(),
            yaw,
            pitch,
          };
          if !meta.fields.is_empty() {
            match metadata(ty, &meta, ver, conn.conv()) {
              Some(data) => {
                return Ok(smallvec![
                  spawn,
                  GPacket::EntityMetadataV8 { entity_id: eid, unknown: data }
                ])
              }
              None => spawn,
            }
          } else {
            spawn
          }
        }
      }
      Packet::SwitchServer { ips } => {
        conn.switch_to(ips);
        return Ok(smallvec![]);
      }
      Packet::Tags { block, item, fluid, entity_type, game_event } => {
        if ver >= ProtocolVersion::V1_14_4 {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_varint(5);
          macro_rules! tag {
            ( $name:expr, $tag:expr ) => {
              buf.write_str($name);
              buf.write_varint($tag.len() as i32);
              for (name, tag) in &$tag {
                buf.write_str(name);
                buf.write_varint(tag.len() as i32);
                for elem in tag {
                  buf.write_varint(*elem);
                }
              }
            };
          }
          tag!("minecraft:block", block);
          tag!("minecraft:item", item);
          tag!("minecraft:fluid", fluid);
          tag!("minecraft:entity_type", entity_type);
          tag!("minecraft:game_event", game_event);
          // GPacket::SynchronizeTagsV14 { unknown: data }
          return Ok(smallvec![]);
        } else {
          return Err(WriteError::InvalidVer);
        }
      }
      Packet::Title { action } => {
        if ver >= ProtocolVersion::V1_17_1 {
          match action {
            TitleAction::Title(chat) => GPacket::TitleV17 { title: chat.to_json() },
            TitleAction::Subtitle(chat) => GPacket::SubtitleV17 { subtitle: chat.to_json() },
            TitleAction::Times { fade_in, stay, fade_out } => GPacket::TitleFadeV17 {
              fade_in_ticks:  fade_in as i32,
              remain_ticks:   stay as i32,
              fade_out_ticks: fade_out as i32,
            },
            TitleAction::Clear(reset) => GPacket::ClearTitleV17 { reset },
          }
        } else {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          match action {
            TitleAction::Title(ref chat) => buf.write_str(&chat.to_json()),
            TitleAction::Subtitle(ref chat) => buf.write_str(&chat.to_json()),
            TitleAction::Times { fade_in, stay, fade_out } => {
              buf.write_i32(fade_in as i32);
              buf.write_i32(stay as i32);
              buf.write_i32(fade_out as i32);
            }
            _ => {}
          }
          GPacket::TitleV8 {
            ty:      if ver >= ProtocolVersion::V1_12_2 {
              match action {
                TitleAction::Title(_) => 0,
                TitleAction::Subtitle(_) => 1,
                TitleAction::Times { .. } => 3,
                TitleAction::Clear(false) => 4,
                TitleAction::Clear(true) => 5,
              }
            } else {
              match action {
                TitleAction::Title(_) => 0,
                TitleAction::Subtitle(_) => 1,
                TitleAction::Times { .. } => 2,
                TitleAction::Clear(false) => 3,
                TitleAction::Clear(true) => 4,
              }
            },
            unknown: data,
          }
        }
      }
      Packet::Teams { team, action } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        fn write_entities(buf: &mut Buffer<&mut Vec<u8>>, entities: &[String]) {
          buf.write_list(entities, |buf, n| buf.write_str(n.as_str()));
        }
        fn write_info(ver: ProtocolVersion, buf: &mut Buffer<&mut Vec<u8>>, info: &TeamInfo) {
          if ver >= ProtocolVersion::V1_14_4 {
            buf.write_str(&info.display_name.to_json());
            buf.write_u8(
              if info.friendly_fire { 0x01 } else { 0x00 }
                | if info.see_invis { 0x02 } else { 0x00 },
            );
            buf.write_str(match info.name_tag {
              TeamRule::Always => "always",
              TeamRule::ForOtherTeams => "hideForOtherTeams",
              TeamRule::ForOwnTeam => "hideForOwnTeam",
              TeamRule::Never => "never",
            });
            buf.write_str(match info.collisions {
              TeamRule::Always => "always",
              TeamRule::ForOtherTeams => "pushrOtherTeams",
              TeamRule::ForOwnTeam => "pushOwnTeam",
              TeamRule::Never => "never",
            });
            buf.write_varint(info.color.id().into());
            buf.write_str(&info.prefix.to_json());
            buf.write_str(&info.postfix.to_json());
          } else if ver >= ProtocolVersion::V1_9_4 {
            buf.write_str(&info.display_name.to_codes());
            // Team colors are broken. This code makes titles match the functionality of
            // 1.14+ clients.
            let mut prefix = info.prefix.to_codes();
            prefix.push(chat::CODE_SEP);
            prefix.push(info.color.code());
            buf.write_str(&prefix);
            buf.write_str(&info.postfix.to_codes());
            buf.write_u8(
              if info.friendly_fire { 0x01 } else { 0x00 }
                | if info.see_invis { 0x02 } else { 0x00 },
            );
            buf.write_str(match info.name_tag {
              TeamRule::Always => "always",
              TeamRule::ForOtherTeams => "hideForOtherTeams",
              TeamRule::ForOwnTeam => "hideForOwnTeam",
              TeamRule::Never => "never",
            });
            buf.write_str(match info.collisions {
              TeamRule::Always => "always",
              TeamRule::ForOtherTeams => "pushrOtherTeams",
              TeamRule::ForOwnTeam => "pushOwnTeam",
              TeamRule::Never => "never",
            });
            // This is pointless, as the client will never render it. But theres no real
            // reason not to send it.
            buf.write_varint(info.color.id().into());
          } else {
            buf.write_str(&info.display_name.to_codes());
            // Team colors are broken. This code makes titles match the functionality of
            // 1.14+ clients.
            let mut prefix = info.prefix.to_codes();
            prefix.push(chat::CODE_SEP);
            prefix.push(info.color.code());
            buf.write_str(&prefix);
            buf.write_str(&info.postfix.to_codes());
            buf.write_u8(
              if info.friendly_fire { 0x01 } else { 0x00 }
                | if info.see_invis { 0x02 } else { 0x00 },
            );
            buf.write_str(match info.name_tag {
              TeamRule::Always => "always",
              TeamRule::ForOtherTeams => "hideForOtherTeams",
              TeamRule::ForOwnTeam => "hideForOwnTeam",
              TeamRule::Never => "never",
            });
            // This is pointless, as the client will never render it. But theres no real
            // reason not to send it.
            buf.write_u8(info.color.id());
          }
        }
        match &action {
          TeamAction::Create { info, entities } => {
            write_info(ver, &mut buf, info);
            write_entities(&mut buf, entities);
          }
          TeamAction::Remove => {}
          TeamAction::UpdateInfo { info } => {
            write_info(ver, &mut buf, info);
          }
          TeamAction::AddEntities { entities } => {
            write_entities(&mut buf, entities);
          }
          TeamAction::RemoveEntities { entities } => {
            write_entities(&mut buf, entities);
          }
        }
        let ty = match action {
          TeamAction::Create { .. } => 0,
          TeamAction::Remove => 1,
          TeamAction::UpdateInfo { .. } => 2,
          TeamAction::AddEntities { .. } => 3,
          TeamAction::RemoveEntities { .. } => 4,
        };
        if ver >= ProtocolVersion::V1_18 {
          GPacket::TeamsV18 { packet_type: ty, team_name: team, unknown: data }
        } else if ver >= ProtocolVersion::V1_17_1 {
          GPacket::TeamsV17 { packet_type: ty, team_name: team, unknown: data }
        } else {
          GPacket::TeamsV8 { field_149314_f: ty, field_149320_a: team, unknown: data }
        }
      }
      Packet::UnloadChunk { pos } => {
        if ver >= ProtocolVersion::V1_9 {
          GPacket::UnloadChunkV9 { x: pos.x(), z: pos.z() }
        } else {
          GPacket::ChunkDataV8 {
            chunk_x:        pos.x(),
            chunk_z:        pos.z(),
            field_149279_g: true,
            // Zero bit mask, then zero length varint
            unknown:        vec![0, 0, 0],
          }
        }
      }
      Packet::UpdateHealth { health, food, saturation } => {
        GPacket::UpdateHealthV8 { health, food_level: food, saturation_level: saturation }
      }
      Packet::UpdateViewPos { pos } => {
        if ver >= ProtocolVersion::V1_14 {
          GPacket::ChunkRenderDistanceCenterV14 { chunk_x: pos.x(), chunk_z: pos.z() }
        } else {
          panic!("cannot send UpdateViewPos for version {}", ver);
        }
      }
      Packet::WindowOpen { wid, ty, size, title } => {
        if ver >= ProtocolVersion::V1_14_4 {
          let id = match ty.as_str() {
            "minecraft:generic_9x1" => 0,
            "minecraft:generic_9x2" => 1,
            "minecraft:generic_9x3" => 2,
            "minecraft:generic_9x4" => 3,
            "minecraft:generic_9x5" => 4,
            "minecraft:generic_9x6" => 5,
            "minecraft:generic_3x3" => 6,
            "minecraft:anvil" => 7,
            "minecraft:beacon" => 8,
            "minecraft:blast_furnace" => 9,
            "minecraft:brewing_stand" => 10,
            "minecraft:crafting" => 11,
            "minecraft:enchantment" => 12,
            "minecraft:furnace" => 13,
            "minecraft:grindstone" => 14,
            "minecraft:hopper" => 15,
            "minecraft:lectern" => 16,
            "minecraft:loom" => 17,
            "minecraft:merchant" => 18,
            "minecraft:shulker_box" => 19,
            "minecraft:smithing" => 20,
            "minecraft:smoker" => 21,
            "minecraft:cartography" => 22,
            "minecraft:stonecutter" => 23,
            _ => 0,
          };
          if ver >= ProtocolVersion::V1_16_5 {
            GPacket::OpenScreenV16 {
              sync_id:           wid.into(),
              screen_handler_id: id,
              name:              title,
            }
          } else {
            GPacket::OpenWindowV14 {
              sync_id:      wid.into(),
              container_id: id,
              name:         title,
            }
          }
        } else {
          GPacket::OpenWindowV8 {
            window_id:      wid.into(),
            inventory_type: ty,
            window_title:   title,
            slot_count:     size as i32,
            unknown:        vec![],
          }
        }
      }
      Packet::WindowItems { wid, items, held } => {
        if ver >= ProtocolVersion::V1_17_1 {
          let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
          buf.write_varint(items.len() as i32);
          for mut it in items {
            conn.conv().item(&mut it, ver.block());
            buf.write_item(&it);
          }
          buf.write_item(&held);
          GPacket::WindowItemsV17 { sync_id: wid.into(), revision: 0, unknown: buf.serialize() }
        } else {
          let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
          buf.write_i16(items.len() as i16);
          for mut it in items {
            conn.conv().item(&mut it, ver.block());
            buf.write_item(&it);
          }
          GPacket::WindowItemsV8 { window_id: wid.into(), unknown: buf.serialize(), v_2: 0 }
        }
      }
      Packet::WindowItem { wid, slot, mut item } => {
        let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
        conn.conv().item(&mut item, ver.block());
        buf.write_item(&item);
        if ver >= ProtocolVersion::V1_17_1 {
          GPacket::SetSlotV17 { sync_id: wid.into(), revision: 0, slot, unknown: buf.serialize() }
        } else {
          GPacket::SetSlotV8 { window_id: wid.into(), slot, unknown: buf.serialize() }
        }
      }
      _ => todo!("convert {:?} into generated packet", self),
    }])
  }
}

#[derive(Debug, Clone, Serialize)]
struct Dimension {
  ambient_light:        f32,
  bed_works:            bool,
  coordinate_scale:     f32,
  effects:              String,
  has_ceiling:          bool,
  has_raids:            bool,
  has_skylight:         bool,
  height:               i32, // 1.17+
  infiniburn:           String,
  logical_height:       i32,
  min_y:                i32, // 1.17+
  natural:              bool,
  piglin_safe:          bool,
  fixed_time:           i64,
  respawn_anchor_works: bool,
  ultrawarm:            bool,

  // 1.19+
  monster_spawn_light_level:       i32,
  monster_spawn_block_light_limit: i32,
}

#[derive(Debug, Clone, Serialize)]
struct Biome {
  category:      String,
  depth:         f32,
  downfall:      f32,
  effects:       BiomeEffects,
  precipitation: String,
  scale:         f32,
  temperature:   f32,
}
#[derive(Debug, Clone, Serialize)]
struct BiomeEffects {
  sky_color:       i32,
  fog_color:       i32,
  water_fog_color: i32,
  water_color:     i32,
  #[serde(skip_serializing_if = "Option::is_none")]
  foliage_color:   Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  grass_color:     Option<i32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  mood_sound:      Option<MoodSound>, // 1.18.2+
}
#[derive(Debug, Clone, Serialize)]
struct MoodSound {
  block_search_extent: i32,
  offset:              f64,
  sound:               String,
  tick_delay:          i32,
}

#[derive(Debug, Clone, Serialize)]
struct LoginInfo {
  #[serde(rename = "minecraft:dimension_type")]
  dimensions: Codec<Dimension>,
  #[serde(rename = "minecraft:worldgen/biome")]
  biomes:     Codec<Biome>,
}
#[derive(Debug, Clone, Serialize)]
struct Codec<T> {
  #[serde(rename = "type")]
  ty:    String,
  value: Vec<CodecItem<T>>,
}
#[derive(Debug, Clone, Serialize)]
struct CodecItem<T> {
  name:    String,
  id:      i32,
  element: T,
}

fn write_dimensions<T>(
  out: &mut Buffer<T>,
  ver: ProtocolVersion,
  world_height: u32,
  world_min_y: i32,
) where
  std::io::Cursor<T>: std::io::Write,
{
  let dimension = Dimension {
    piglin_safe:          false,
    natural:              true,
    ambient_light:        0.0,
    fixed_time:           6000,
    infiniburn:           "#minecraft:infiniburn_overworld".into(),
    respawn_anchor_works: false,
    has_skylight:         true,
    bed_works:            true,
    effects:              "minecraft:overworld".into(),
    has_raids:            false,
    logical_height:       128,
    coordinate_scale:     1.0,
    ultrawarm:            false,
    has_ceiling:          false,
    min_y:                world_min_y,
    height:               (world_height as i32 + 15) / 16 * 16,

    monster_spawn_light_level:       7,
    monster_spawn_block_light_limit: 7,
  };
  let biome = Biome {
    precipitation: "rain".into(),
    depth:         1.0,
    temperature:   1.0,
    scale:         1.0,
    downfall:      1.0,
    category:      "none".into(),
    effects:       BiomeEffects {
      sky_color:       0x78a7ff,
      fog_color:       0xc0d8ff,
      water_fog_color: 0x050533,
      water_color:     0x3f76e4,
      foliage_color:   None,
      grass_color:     None,
      mood_sound:      Some(MoodSound {
        block_search_extent: 8,
        offset:              2.0,
        sound:               "minecraft:ambient.cave".into(),
        tick_delay:          6000,
      }),
      // sky_color:       0xff00ff,
      // water_color:     0xff00ff,
      // fog_color:       0xff00ff,
      // water_fog_color: 0xff00ff,
      // grass_color:     0xff00ff,
      // foliage_color:   0x00ffe5,
      // grass_color:     0xff5900,
    },
  };
  let dimension_tag = nbt::to_nbt("", &dimension).unwrap();

  let info = LoginInfo {
    dimensions: Codec {
      ty:    "minecraft:dimension_type".into(),
      value: vec![CodecItem {
        name:    "minecraft:overworld".into(),
        id:      0,
        element: dimension,
      }],
    },
    biomes:     Codec {
      ty:    "minecraft:worldgen/biome".into(),
      value: vec![CodecItem { name: "minecraft:plains".into(), id: 0, element: biome }],
    },
  };

  // Dimension codec
  out.write_buf(&nbt::to_nbt("", &info).unwrap().serialize());
  if ver >= ProtocolVersion::V1_19 {
    // Current dimension type (key in dimension codec)
    out.write_str("minecraft:overworld");
    // Current world
    out.write_str("minecraft:overworld");
  } else {
    // World codec (included in dimension)
    out.write_buf(&dimension_tag.serialize());
    // Current world
    out.write_str("minecraft:overworld");
  }
}

#[test]
fn test_codec() {
  use bb_common::nbt::Tag;

  let expected = Tag::compound(&[
    ("piglin_safe", Tag::Byte(0)),
    ("natural", Tag::Byte(1)),
    ("ambient_light", Tag::Float(0.0)),
    ("fixed_time", Tag::Long(6000)),
    ("infiniburn", Tag::String("".into())),
    ("respawn_anchor_works", Tag::Byte(0)),
    ("has_skylight", Tag::Byte(1)),
    ("bed_works", Tag::Byte(1)),
    ("effects", Tag::String("minecraft:overworld".into())),
    ("has_raids", Tag::Byte(0)),
    ("logical_height", Tag::Int(128)),
    ("coordinate_scale", Tag::Float(1.0)),
    ("ultrawarm", Tag::Byte(0)),
    ("has_ceiling", Tag::Byte(0)),
    // 1.17+
    ("min_y", Tag::Int(0)),
    ("height", Tag::Int(256)),
  ]);
  let dimension = Dimension {
    piglin_safe:          false,
    natural:              true,
    ambient_light:        0.0,
    fixed_time:           6000,
    infiniburn:           "".into(),
    respawn_anchor_works: false,
    has_skylight:         true,
    bed_works:            true,
    effects:              "minecraft:overworld".into(),
    has_raids:            false,
    logical_height:       128,
    coordinate_scale:     1.0,
    ultrawarm:            false,
    has_ceiling:          false,
    min_y:                0,
    height:               256,
  };
  assert_eq!(expected, nbt::to_tag(&dimension).unwrap());
  let expected = Tag::compound(&[
    ("precipitation", Tag::String("rain".into())),
    ("depth", Tag::Float(1.0)),
    ("temperature", Tag::Float(1.0)),
    ("scale", Tag::Float(1.0)),
    ("downfall", Tag::Float(1.0)),
    ("category", Tag::String("none".into())),
    (
      "effects",
      Tag::compound(&[
        ("sky_color", Tag::Int(0x78a7ff)),
        ("fog_color", Tag::Int(0xc0d8ff)),
        ("water_fog_color", Tag::Int(0x050533)),
        ("water_color", Tag::Int(0x3f76e4)),
        // ("sky_color", Tag::Int(0xff00ff)),
        // ("water_color", Tag::Int(0xff00ff)),
        // ("fog_color", Tag::Int(0xff00ff)),
        // ("water_fog_color", Tag::Int(0xff00ff)),
        // ("grass_color", Tag::Int(0xff00ff)),
        // ("foliage_color", Tag::Int(0x00ffe5)),
        // ("grass_color", Tag::Int(0xff5900)),
      ]),
    ),
  ]);
  let biome = Biome {
    precipitation: "rain".into(),
    depth:         1.0,
    temperature:   1.0,
    scale:         1.0,
    downfall:      1.0,
    category:      "none".into(),
    effects:       BiomeEffects {
      sky_color:       0x78a7ff,
      fog_color:       0xc0d8ff,
      water_fog_color: 0x050533,
      water_color:     0x3f76e4,
      foliage_color:   None,
      grass_color:     None,
      mood_sound:      None,
      // sky_color:       0xff00ff,
      // water_color:     0xff00ff,
      // fog_color:       0xff00ff,
      // water_fog_color: 0xff00ff,
      // grass_color:     0xff00ff,
      // foliage_color:   0x00ffe5,
      // grass_color:     0xff5900,
    },
  };
  dbg!(&expected);
  dbg!(&nbt::to_tag(&biome).unwrap());
  assert_eq!(expected, nbt::to_tag(&biome).unwrap());
}

fn object_ty(entity: i32) -> i32 {
  // I cannot find the normal entity ids for these objects:
  // _ => 11,   // Minecart (storage, unused)
  // _ => 12,   // Minecart (powered, unused)
  // _ => 74,   // Falling Dragon Egg
  // _ => 90,   // Fishing Float
  // _ => 92,  // Tipped Arrow
  match entity {
    41 => 1,   // Boat
    1 => 2,    // Item Stack (Slot)
    3 => 3,    // Area Effect Cloud
    42 => 10,  // Minecart
    20 => 50,  // Activated TNT
    200 => 51, // EnderCrystal
    10 => 60,  // Arrow (projectile)
    11 => 61,  // Snowball (projectile)
    7 => 62,   // Egg (projectile)
    12 => 63,  // FireBall (ghast projectile)
    13 => 64,  // FireCharge (blaze projectile)
    14 => 65,  // Thrown Enderpearl
    19 => 66,  // Wither Skull (projectile)
    25 => 67,  // Shulker Bullet
    21 => 70,  // Falling Objects
    18 => 71,  // Item frames
    15 => 72,  // Eye of Ender
    16 => 73,  // Thrown Potion
    17 => 75,  // Thrown Exp Bottle
    22 => 76,  // Firework Rocket
    8 => 77,   // Leash Knot
    30 => 78,  // ArmorStand
    24 => 91,  // Spectral Arrow
    26 => 93,  // Dragon Fireball
    _ => panic!("not an object: {entity}"),
  }
}
