use super::{metadata, TypeConverter};
use crate::{
  gnet::{cb::Packet as GPacket, tcp},
  stream::PacketStream,
  Conn,
};
use sc_common::{
  nbt,
  net::{
    cb,
    cb::{
      CommandType, ObjectiveAction, ObjectiveType, Packet, ScoreboardAction, ScoreboardDisplay,
    },
  },
  util::{Buffer, UUID},
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
          }
        } else {
          GPacket::PlayerAbilitiesV16 {
            invulnerable,
            flying,
            allow_flying,
            creative_mode: insta_break,
            fly_speed: fly_speed * 0.05,
            walk_speed: walk_speed * 0.1,
          }
        },
      Packet::BlockUpdate { pos, state } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        buf.write_varint(state as i32);
        GPacket::BlockUpdateV8 { block_position: pos, block_state: None, unknown: data }
      }
      Packet::Chat { msg, ty } => {
        if ver < ProtocolVersion::V1_12_2 {
          GPacket::ChatV8 { chat_component: msg, ty: ty as i8 }
        } else if ver < ProtocolVersion::V1_16_5 {
          GPacket::ChatV12 { chat_component: msg, ty: None, unknown: vec![ty] }
        } else {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_u8(ty);
          buf.write_uuid(UUID::from_u128(0));
          GPacket::ChatV12 { chat_component: msg, ty: None, unknown: data }
        }
      }
      Packet::Chunk { pos, full, bit_map, sections, sky_light, block_light } => {
        return Ok(super::chunk(
          pos,
          full,
          bit_map,
          sections,
          sky_light,
          block_light,
          ver,
          conn.conv(),
        ));
      }
      Packet::CommandList { nodes, root } => {
        if ver < ProtocolVersion::V1_13 {
          panic!("command tree doesn't exist for version {}", ver);
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
            buf.write_str(&node.parser);
            buf.write_buf(&node.properties);
          }
          if let Some(suggestion) = &node.suggestion {
            buf.write_str(&suggestion);
          }
        });
        buf.write_varint(root as i32);
        if ver > ProtocolVersion::V1_14_4 {
          GPacket::CommandTreeV16 { command_tree: None, unknown: data }
        } else {
          GPacket::CommandTreeV14 { command_tree: None, unknown: data }
        }
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
        } else if ver < ProtocolVersion::V1_17_1 {
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
        } else {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_varint(eid);
          buf.write_i16(x);
          buf.write_i16(y);
          buf.write_i16(z);
          buf.write_bool(on_ground);
          GPacket::EntityRelMoveV17 { unknown: data }
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
        } else if ver < ProtocolVersion::V1_17_1 {
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
        } else {
          let mut data = vec![];
          let mut buf = Buffer::new(&mut data);
          buf.write_varint(eid);
          buf.write_i16(x.into());
          buf.write_i16(y.into());
          buf.write_i16(z.into());
          buf.write_i8(yaw);
          buf.write_i8(pitch);
          buf.write_bool(on_ground);
          GPacket::EntityLookMoveV17 { unknown: data }
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
      } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        if ver >= ProtocolVersion::V1_18 {
          buf.write_i32(eid);
          buf.write_bool(hardcore_mode);
        }
        if ver >= ProtocolVersion::V1_16_5 {
          buf.write_u8(game_mode.id());
          buf.write_i8(-1); // no previous_game_mode

          // List of worlds
          buf.write_varint(1);
          buf.write_str("minecraft:overworld");

          write_dimensions(&mut buf);

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
        } else {
          if ver >= ProtocolVersion::V1_15_2 {
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
            reduced_debug_info: None,
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
            reduced_debug_info: None,
            unknown: data,
          },
          14..=15 => GPacket::JoinGameV14 {
            player_entity_id:    eid,
            hardcore:            hardcore_mode,
            game_mode:           None,
            dimension:           None,
            max_players:         None,
            generator_type:      None,
            chunk_load_distance: None,
            reduced_debug_info:  None,
            unknown:             data,
          },
          16 => GPacket::JoinGameV16 {
            player_entity_id:   eid,
            hardcore:           hardcore_mode,
            sha_256_seed:       None,
            game_mode:          None,
            previous_game_mode: None,
            dimension_ids:      None,
            registry_manager:   None,
            dimension_type:     None,
            dimension_id:       None,
            max_players:        None,
            view_distance:      None,
            reduced_debug_info: None,
            show_death_screen:  None,
            debug_world:        None,
            flat_world:         None,
            unknown:            data,
          },
          17 => GPacket::JoinGameV17 {
            a:                  None,
            player_entity_id:   eid,
            hardcore:           hardcore_mode,
            sha_256_seed:       None,
            game_mode:          None,
            previous_game_mode: None,
            dimension_ids:      None,
            registry_manager:   None,
            dimension_type:     None,
            dimension_id:       None,
            max_players:        None,
            view_distance:      None,
            reduced_debug_info: None,
            show_death_screen:  None,
            debug_world:        None,
            flat_world:         None,
            unknown:            data,
          },
          18 => GPacket::JoinGameV18 {
            l:                  None,
            player_entity_id:   None,
            hardcore:           None,
            sha_256_seed:       None,
            game_mode:          None,
            previous_game_mode: None,
            dimension_ids:      None,
            registry_manager:   None,
            dimension_type:     None,
            dimension_id:       None,
            max_players:        None,
            view_distance:      None,
            reduced_debug_info: None,
            show_death_screen:  None,
            debug_world:        None,
            flat_world:         None,
            unknown:            data,
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
        if ver < ProtocolVersion::V1_17_1 {
          GPacket::PlayerListV8 { action: id, players: None, unknown: data }
        } else {
          GPacket::PlayerListV17 { action: id, entries: None, unknown: data }
        }
      }
      Packet::PluginMessage { channel, data } => {
        // No length prefix for data, it is infered from packet length.
        if ver < ProtocolVersion::V1_14_4 {
          GPacket::CustomPayloadV8 { channel, data: None, unknown: data }
        } else {
          GPacket::CustomPayloadV14 {
            brand:                  None,
            debug_path:             None,
            debug_neighbors_update: None,
            debug_structures:       None,
            debug_worldgen_attempt: None,
            debug_poi_ticket_count: None,
            debug_poi_added:        None,
            debug_poi_removed:      None,
            debug_village_sections: None,
            debug_goal_selector:    None,
            debug_brain:            None,
            debug_caves:            None,
            debug_raids:            None,
            channel:                channel,
            data:                   None,
            unknown:                data,
          }
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
            buf.write_str(&value);
            buf.write_varint(match ty {
              ObjectiveType::Integer => 0,
              ObjectiveType::Hearts => 1,
            });
          }
          _ => {}
        }
        if ver < ProtocolVersion::V1_18 {
          GPacket::ScoreboardObjectiveV8 {
            objective_name:  objective,
            field_149342_c:  m,
            objective_value: None,
            ty:              None,
            unknown:         data,
          }
        } else {
          GPacket::ScoreboardObjectiveV18 {
            name:         objective,
            mode:         m,
            unknown:      data,
            add_mode:     None,
            remove_mode:  None,
            update_mode:  None,
            display_name: None,
            ty:           None,
          }
        }
      }
      Packet::ScoreboardUpdate { username, objective, action } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
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
          value: None,
          unknown: data,
        }
      }
      Packet::SetPosLook { x, y, z, yaw, pitch, flags, teleport_id, should_dismount } => {
        let mut data = vec![];
        let mut buf = Buffer::new(&mut data);
        buf.write_u8(flags);
        if ver >= ProtocolVersion::V1_9 {
          buf.write_varint(teleport_id as i32);
        }
        if ver >= ProtocolVersion::V1_17_1 {
          buf.write_bool(should_dismount);
        }
        GPacket::PlayerPosLookV8 { x, y, z, yaw, pitch, field_179835_f: None, unknown: data }
      }
      Packet::SpawnLivingEntity {
        eid,
        id,
        ty,
        x,
        y,
        z,
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
          GPacket::SpawnMobV15 {
            id: eid,
            uuid: id,
            entity_type_id: ty,
            x,
            y,
            z,
            velocity_x: vel_x.into(),
            velocity_y: vel_y.into(),
            velocity_z: vel_z.into(),
            yaw,
            pitch,
            head_yaw,
          }
        } else if ver >= ProtocolVersion::V1_11 {
          GPacket::SpawnMobV11 {
            entity_id: eid,
            unique_id: id,
            ty,
            x,
            y,
            z,
            velocity_x: vel_x.into(),
            velocity_y: vel_y.into(),
            velocity_z: vel_z.into(),
            yaw,
            pitch,
            head_pitch: head_yaw,
            data_manager: None,
            data_manager_entries: None,
            unknown: metadata(new_ty, &meta, ver, conn.conv()),
          }
        } else if ver >= ProtocolVersion::V1_9 {
          GPacket::SpawnMobV9 {
            entity_id: eid,
            unique_id: id,
            ty,
            x,
            y,
            z,
            velocity_x: vel_x.into(),
            velocity_y: vel_y.into(),
            velocity_z: vel_z.into(),
            yaw,
            pitch,
            head_pitch: head_yaw,
            data_manager: None,
            data_manager_entries: None,
            unknown: vec![0xff], // No entity metadata
          }
        } else {
          GPacket::SpawnMobV8 {
            entity_id: eid,
            ty,
            x: (x * 32.0) as i32,
            y: (y * 32.0) as i32,
            z: (z * 32.0) as i32,
            velocity_x: vel_x.into(),
            velocity_y: vel_y.into(),
            velocity_z: vel_z.into(),
            yaw,
            pitch,
            head_pitch: head_yaw,
            field_149043_l: None,
            watcher: None,
            unknown: metadata(new_ty, &meta, ver, conn.conv()),
          }
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
      Packet::SwitchServer { ips } => {
        conn.switch_to(ips);
        return Ok(smallvec![]);
      }
      Packet::UnloadChunk { pos } => {
        if ver >= ProtocolVersion::V1_9 {
          GPacket::UnloadChunkV9 { x: pos.x(), z: pos.z() }
        } else {
          GPacket::ChunkDataV8 {
            chunk_x:        pos.x(),
            chunk_z:        pos.z(),
            field_149279_g: true,
            extracted_data: None,
            // Zero bit mask, then zero length varint
            unknown:        vec![0, 0, 0],
          }
        }
      }
      Packet::UpdateViewPos { pos } => {
        if ver >= ProtocolVersion::V1_14 {
          GPacket::ChunkRenderDistanceCenterV14 { chunk_x: pos.x(), chunk_z: pos.z() }
        } else {
          panic!("cannot send UpdateViewPos for version {}", ver);
        }
      }
      Packet::WindowOpen { wid, ty, title } => {
        GPacket::OpenWindowV8 {
          window_id:      wid.into(),
          inventory_type: "minecraft:chest".into(),
          window_title:   title,
          slot_count:     (ty * 9).into(),
          entity_id:      None,
          unknown:        vec![],
        }
      }
      Packet::WindowItems { wid, items, held } => {
        let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
        buf.write_i16(items.len() as i16);
        for mut it in items {
          let (id, damage) = conn.conv().item_to_old(it.id as u32, ver.block());
          it.id = id as i32;
          it.damage = damage as i16;
          buf.write_item(&it);
        }
        GPacket::WindowItemsV8 {
          window_id:   wid.into(),
          item_stacks: vec![],
          unknown:     buf.serialize(),
        }
      }
      Packet::WindowItem { wid, slot, mut item } => {
        let mut buf = tcp::Packet::from_buf_id(vec![], 0, ver);
        let (id, damage) = conn.conv().item_to_old(item.id as u32, ver.block());
        item.id = id as i32;
        item.damage = damage as i16;
        buf.write_item(&item);
        GPacket::SetSlotV8 {
          window_id: wid.into(),
          slot:      slot,
          item:      None,
          unknown:   buf.serialize(),
        }
      }
      _ => todo!("convert {:?} into generated packet", self),
    }])
  }
}

#[derive(Debug, Clone, Serialize)]
struct Dimension {
  piglin_safe:          bool,
  natural:              bool,
  ambient_light:        f32,
  fixed_time:           i64,
  infiniburn:           String,
  respawn_anchor_works: bool,
  has_skylight:         bool,
  bed_works:            bool,
  effects:              String,
  has_raids:            bool,
  logical_height:       i32,
  coordinate_scale:     f32,
  ultrawarm:            bool,
  has_ceiling:          bool,
  // 1.17+
  min_y:                i32,
  height:               i32,
}

#[derive(Debug, Clone, Serialize)]
struct Biome {
  precipitation: String,
  depth:         f32,
  temperature:   f32,
  scale:         f32,
  downfall:      f32,
  category:      String,
  effects:       BiomeEffects,
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

fn write_dimensions<T>(out: &mut Buffer<T>)
where
  std::io::Cursor<T>: std::io::Write,
{
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

  out.write_buf(&nbt::to_nbt("", &info).unwrap().serialize());
  out.write_buf(&dimension_tag.serialize());
  // Current world
  out.write_str("minecraft:overworld");
}

#[test]
fn test_codec() {
  use sc_common::nbt::Tag;

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
