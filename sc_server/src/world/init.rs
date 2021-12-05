use super::World;
use crate::{
  command::{Command, Parser, StringType},
  player::Player,
};
use rayon::prelude::*;
use sc_common::{
  math::ChunkPos,
  net::cb,
  util::{
    nbt::{Tag, NBT},
    Buffer, Chat,
  },
  version::ProtocolVersion,
};

impl World {
  pub fn init(&self) {
    let mut c = Command::new("say");
    c.add_arg("text", Parser::String(StringType::Greedy));
    self.commands().add(c, |world, _, args| {
      world.broadcast(format!("[Server] {}", args[1].str()).as_str());
    });

    let mut c = Command::new("fill");
    c.add_lit("rect")
      .add_arg("min", Parser::BlockPos)
      .add_arg("max", Parser::BlockPos)
      .add_arg("block", Parser::BlockState);
    c.add_lit("circle")
      .add_arg("center", Parser::BlockPos)
      .add_arg("radius", Parser::Float { min: Some(0.0), max: None })
      .add_arg("block", Parser::BlockState);
    c.add_lit("sphere")
      .add_arg("center", Parser::BlockPos)
      .add_arg("radius", Parser::Float { min: Some(0.0), max: None })
      .add_arg("block", Parser::BlockState);
    self.commands().add(c, |world, _, args| {
      // args[0] is `fill`
      match args[1].lit() {
        "rect" => {
          let min = args[2].pos();
          let max = args[3].pos();
          let block = args[4].block();
          let (min, max) = min.min_max(max);
          let w = world.default_world();
          w.fill_rect_kind(min, max, block).unwrap();
        }
        "circle" => {
          let pos = args[2].pos();
          let radius = args[3].float();
          let block = args[4].block();
          let w = world.default_world();
          w.fill_circle_kind(pos, radius, block).unwrap();
        }
        "sphere" => {
          let pos = args[2].pos();
          let radius = args[3].float();
          let block = args[4].block();
          let w = world.default_world();
          w.fill_sphere_kind(pos, radius, block).unwrap();
        }
        _ => unreachable!(),
      }
    });
    let mut c = Command::new("flyspeed");
    c.add_arg("multiplier", Parser::Float { min: Some(0.0), max: None });
    self.commands().add(c, |_, player, args| {
      // args[0] is `flyspeed`
      let v = args[1].float();
      if let Some(p) = player {
        p.set_flyspeed(v);
      }
    });
    let mut c = Command::new("summon");
    c.add_arg("entity", Parser::EntitySummon);
    self.commands().add(c, |_, player, args| {
      // args[0] is `summon`
      let ty = args[1].entity_summon();
      if let Some(p) = player {
        let eid = p.world().summon(ty, p.pos());
        info!("eid of mob: {}", eid);
        p.send_message(&Chat::new(format!("summoned {:?}", ty)));
      }
    });

    info!("generating terrain...");
    (-10..=10).into_par_iter().for_each(|x| {
      for z in -10..=10 {
        self.chunk(ChunkPos::new(x, z), |_| {});
      }
    });
    info!("done generating terrain");
  }

  pub(super) fn player_init(&self, player: &Player) {
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
    let mut world_names = Buffer::new(vec![]);
    world_names.write_varint(1);
    world_names.write_str("minecraft:overworld");
    let out = cb::Packet::JoinGameV8 {
      // entity_id:                self.eid(),
      // game_mode:                1,       // Creative
      // difficulty_removed_v1_14: Some(1), // Normal
      // dimension_v1_8:           Some(0), // Overworld
      // dimension_v1_9_2:         Some(0), // Overworld
      // level_type_removed_v1_16: Some("default".into()),
      // max_players_v1_8:         Some(0), // Ignored
      // max_players_v1_16_2:      Some(0), // Not sure if ignored
      // reduced_debug_info:       false,   // Don't reduce debug info
      //
      // // 1.14+
      // view_distance_v1_14: Some(10), // 10 chunk view distance TODO: Per player view distance
      //
      // // 1.15+
      // hashed_seed_v1_15:           Some(0),
      // enable_respawn_screen_v1_15: Some(true),
      //
      // // 1.16+
      // is_hardcore_v1_16_2:      Some(false),
      // is_flat_v1_16:            Some(false), // Changes the horizon line
      // previous_game_mode_v1_16: Some(1),
      // world_name_v1_16:         Some("overworld".into()),
      // is_debug_v1_16:           Some(false), /* This is not reduced_debug_info, this is for the
      //                                         * world being a debug world */
      // dimension_codec_v1_16:    Some(codec.serialize()),
      // dimension_v1_16:          Some("".into()),
      // dimension_v1_16_2:        Some(NBT::new("", dimension).serialize()),
      // world_names_v1_16:        Some(world_names.into_inner()),
      entity_id:          self.eid(),
      hardcore_mode:      None,
      game_type:          None,
      dimension:          Some(0), // Overworld
      difficulty:         Some(1), // Normal
      max_players:        None,    // Ignored
      world_type:         None,
      reduced_debug_info: Some(false),
      unknown:            vec![],
    };

    player.send(out);
    if player.ver() >= ProtocolVersion::V1_13 {
      player.send(self.commands().serialize());
    }

    let view_distance = 10;
    for x in -view_distance..=view_distance {
      for z in -view_distance..=view_distance {
        let pos = ChunkPos::new(x, z);
        self.inc_view(pos);
        player.send(self.serialize_chunk(pos, player.ver().block()));
      }
    }

    player.send(cb::Packet::SpawnPositionV8 {
      // x:                0.0,        // X
      // y:                60.0,       // Y
      // z:                0.0,        // Z
      // yaw:              0.0,        // Yaw
      // pitch:            0.0,        // Pitch
      // flags:            0,          // Flags
      // teleport_id_v1_9: Some(1234), // TP id
      spawn_block_pos: None,
      unknown:         vec![],
    });

    /*
    let mut info = Buffer::new(vec![]);
    let mut num_info = 1;

    info.write_buf(&player.id().as_be_bytes());
    info.write_str(player.username());
    info.write_varint(0); // no properties
    info.write_varint(1); // creative
    info.write_varint(50); // ping
    info.write_bool(false); // no display name follows

    let mut my_info = Buffer::new(vec![]);
    my_info.write_varint(1); // just 1 player (me)
    my_info.write_buf(&player.id().as_be_bytes());
    my_info.write_str(player.username());
    my_info.write_varint(0); // no properties
    my_info.write_varint(1); // creative
    my_info.write_varint(50); // ping
    my_info.write_bool(false); // no display name follows
    let my_info = cb::Packet::PlayerInfo { action: 0, data: my_info.into_inner() };

    let mut spawn_packets = vec![];
    for other in self.players().iter().in_view(ChunkPos::new(0, 0)).not(player.id()) {
      // Lets the other players know that I exist
      other.send(my_info.clone());

      // Create a packet that will spawn player for other
      let (pos, pitch, yaw) = player.pos_look();
      other.send(cb::Packet::NamedEntitySpawn {
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
      });

      // Add other to the list of players that player knows about
      num_info += 1;
      info.write_buf(&other.id().as_be_bytes());
      info.write_str(other.username());
      info.write_varint(0); // no properties
      info.write_varint(1); // creative
      info.write_varint(50); // ping
      info.write_bool(false); // no display name follows

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
    let mut data = Buffer::new(Vec::with_capacity(info.len()));
    data.write_varint(num_info);
    data.write_buf(&info.into_inner());
    player.send(cb::Packet::PlayerListV8 { action: 0, players: Some(vec![]), unknown: vec![] });
    // Need to send the player info before the spawn packets
    for p in spawn_packets {
      player.send(p);
    }
    */
  }
}
