use super::World;
use crate::{
  command::{Command, Parser, StringType},
  net::Connection,
  player::Player,
};
use common::{
  math::ChunkPos,
  net::cb,
  util::nbt::{Tag, NBT},
  version::ProtocolVersion,
};
use rayon::prelude::*;

impl World {
  pub async fn init(&self) {
    let mut c = Command::new("say");
    c.add_arg("text", Parser::String(StringType::Greedy));
    self
      .get_commands()
      .add(c, |world, _| async move {
        world.broadcast("[Server] big announce").await;
      })
      .await;

    let mut c = Command::new("fill");
    c.add_lit("rect")
      .add_arg("min", Parser::BlockPos)
      .add_arg("max", Parser::BlockPos)
      .add_arg("block", Parser::BlockState);
    c.add_lit("circle")
      .add_arg("center", Parser::BlockPos)
      .add_arg("radius", Parser::Float { min: Some(0.0), max: None })
      .add_arg("block", Parser::BlockState);
    self
      .get_commands()
      .add(c, |world, _| async move {
        world.broadcast("called /fill").await;
      })
      .await;

    info!("generating terrain...");
    (-10..=10).into_par_iter().for_each(|x| {
      for z in -10..=10 {
        self.chunk(ChunkPos::new(x, z), |_| {});
      }
    });
    info!("done generating terrain");
  }

  pub(super) async fn player_init(&self, player: &Player, conn: &Connection) {
    let dimension = Tag::compound(&[
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
    ]);
    let biome = Tag::compound(&[
      ("precipitation", Tag::String("rain".into())),
      ("depth", Tag::Float(1.0)),
      ("temperature", Tag::Float(1.0)),
      ("scale", Tag::Float(1.0)),
      ("downfall", Tag::Float(1.0)),
      ("category", Tag::String("none".into())),
      (
        "effects",
        Tag::compound(&[
          ("sky_color", Tag::Int(0xff00ff)),
          ("water_color", Tag::Int(0xff00ff)),
          ("fog_color", Tag::Int(0xff00ff)),
          ("water_fog_color", Tag::Int(0xff00ff)),
        ]),
      ),
    ]);
    let codec = NBT::new(
      "",
      Tag::compound(&[
        (
          "minecraft:dimension_type",
          Tag::compound(&[
            ("type", Tag::String("minecraft:dimension_type".into())),
            (
              "value",
              Tag::List(vec![Tag::compound(&[
                ("name", Tag::String("minecraft:overworld".into())),
                ("id", Tag::Int(0)),
                ("element", dimension.clone()),
              ])]),
            ),
          ]),
        ),
        (
          "minecraft:worldgen/biome",
          Tag::compound(&[
            ("type", Tag::String("minecraft:worldgen/biome".into())),
            (
              "value",
              Tag::List(vec![Tag::compound(&[
                ("name", Tag::String("minecraft:plains".into())),
                ("id", Tag::Int(0)),
                ("element", biome),
              ])]),
            ),
          ]),
        ),
      ]),
    );
    let out = cb::Packet::Login {
      entity_id:                self.eid(),
      game_mode:                1,       // Creative
      difficulty_removed_v1_14: Some(1), // Normal
      dimension_v1_8:           Some(0), // Overworld
      dimension_v1_9_2:         Some(0), // Overworld
      level_type_removed_v1_16: Some("default".into()),
      max_players_v1_8:         Some(0), // Ignored
      max_players_v1_16_2:      Some(0), // Not sure if ignored
      reduced_debug_info:       false,   // Don't reduce debug info

      // 1.14+
      view_distance_v1_14: Some(10), // 10 chunk view distance TODO: Per player view distance

      // 1.15+
      hashed_seed_v1_15:           Some(0),
      enable_respawn_screen_v1_15: Some(true),

      // 1.16+
      is_hardcore_v1_16_2:      Some(false),
      is_flat_v1_16:            Some(false), // Changes the horizon line
      previous_game_mode_v1_16: Some(1),
      world_name_v1_16:         Some("overworld".into()),
      is_debug_v1_16:           Some(false), /* This is not reduced_debug_info, this is for the
                                              * world being a debug world */
      dimension_codec_v1_16:    Some(codec.serialize()),
      dimension_v1_16:          Some("".into()),
      dimension_v1_16_2:        Some(vec![]),
      // dimension: NBT::new("", dimension).serialize(),
      // world_names:        vec!["minecraft:overworld".into()],
      world_names_v1_16:        Some(vec![]),
    };

    conn.send(out).await;
    if player.ver() >= ProtocolVersion::V1_13 {
      conn.send(self.get_commands().serialize().await).await;
    }

    for x in -10..=10 {
      for z in -10..=10 {
        conn.send(self.serialize_chunk(ChunkPos::new(x, z), player.ver().block())).await;
      }
    }

    conn
      .send(cb::Packet::Position {
        x:                0.0,        // X
        y:                60.0,       // Y
        z:                0.0,        // Z
        yaw:              0.0,        // Yaw
        pitch:            0.0,        // Pitch
        flags:            0,          // Flags
        teleport_id_v1_9: Some(1234), // TP id
      })
      .await;

    // let mut info = PlayerList {
    //   action: player_list::Action::AddPlayer.into(),
    //   players: vec![player_list::Player {
    //     uuid:             Some(player.id().as_proto()),
    //     name:             player.username().into(),
    //     properties:       vec![],
    //     gamemode:         1,
    //     ping:             300, // TODO: Ping
    //     has_display_name: false,
    //     display_name:     "".into(),
    //   }],
    //   ..Default::default()
    // };
    let mut spawn_packets = vec![];
    for other in self.players().await.iter().in_view(ChunkPos::new(0, 0)).not(player.id()) {
      // Add player to the list of players that other knows about
      // let mut out = cb::Packet::PlayerInfo {
      //   action: player_list::Action::AddPlayer.into(),
      //   // TODO: Fill data
      //   data:   vec![],
      // };
      // out
      //   .set_other(Other::PlayerList(PlayerList {
      //     action:  player_list::Action::AddPlayer.into(),
      //     players: vec![player_list::Player {
      //       uuid:             Some(player.id().as_proto()),
      //       name:             player.username().into(),
      //       properties:       vec![],
      //       gamemode:         1,
      //       ping:             300, // TODO: Ping
      //       has_display_name: false,
      //       display_name:     "".into(),
      //     }],
      //   }))
      //   .unwrap();
      // other.conn().send(out).await;
      // Create a packet that will spawn player for other
      let (pos, pitch, yaw) = player.pos_look();
      other
        .conn()
        .send(cb::Packet::NamedEntitySpawn {
          entity_id:                 player.eid(),
          player_uuid:               player.id(),
          x_v1_8:                    Some(pos.fixed_x()),
          x_v1_9:                    Some(pos.x()),
          y_v1_8:                    Some(pos.fixed_y()),
          y_v1_9:                    Some(pos.y()),
          z_v1_8:                    Some(pos.fixed_z()),
          z_v1_9:                    Some(pos.z()),
          yaw:                       yaw as i8, // TODO: Fix doubles/bytes on 1.8
          pitch:                     pitch as i8,
          current_item_removed_v1_9: Some(0),
          metadata_removed_v1_15:    Some(player.metadata(other.ver()).serialize()),
        })
        .await;

      // Add other to the list of players that player knows about
      // info.players.push(player_list::Player {
      //   uuid:             Some(other.id().as_proto()),
      //   name:             other.username().into(),
      //   properties:       vec![],
      //   gamemode:         1,
      //   ping:             300,
      //   has_display_name: false,
      //   display_name:     "".into(),
      // });
      // Create a packet that will spawn other for player
      let (pos, pitch, yaw) = other.pos_look();
      spawn_packets.push(cb::Packet::NamedEntitySpawn {
        entity_id:                 other.eid(),
        player_uuid:               other.id(),
        x_v1_8:                    Some(pos.fixed_x()),
        x_v1_9:                    Some(pos.x()),
        y_v1_8:                    Some(pos.fixed_y()),
        y_v1_9:                    Some(pos.y()),
        z_v1_8:                    Some(pos.fixed_z()),
        z_v1_9:                    Some(pos.z()),
        yaw:                       yaw as i8,
        pitch:                     pitch as i8,
        current_item_removed_v1_9: Some(0),
        metadata_removed_v1_15:    Some(other.metadata(player.ver()).serialize()),
      });
    }
    // Need to send the player info before the spawn packets
    // let mut out = cb::Packet::new(cb::ID::PlayerInfo);
    // out.set_other(Other::PlayerList(info)).unwrap();
    // conn.send(out).await;
    // for p in spawn_packets {
    //   conn.send(p).await;
    // }

    conn
      .send(cb::Packet::Abilities {
        flags:         0x04 | 0x08,
        flying_speed:  10.0 * 0.05,
        walking_speed: 0.1,
      })
      .await;
  }
}
