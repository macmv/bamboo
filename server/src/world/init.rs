use super::World;
use crate::{
  command::{Command, Parser, StringType},
  net::Connection,
  player::Player,
};
use common::{
  math::ChunkPos,
  net::{cb, Other},
  proto::{player_list, PlayerList},
  util::nbt::{Tag, NBT},
  version::ProtocolVersion,
};

impl World {
  pub async fn init(&self) {
    let mut c = Command::new("say");
    c.add_arg("text", Parser::String(StringType::Greedy));
    self
      .get_commands()
      .add(
        c,
        Box::new(|world, _| {
          Box::pin(async move {
            world.broadcast("[Server] big announce").await;
          })
        }),
      )
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
      .add(
        c,
        Box::new(|world, _| {
          Box::pin(async move {
            world.broadcast("called /fill").await;
          })
        }),
      )
      .await;

    info!("generating terrain...");
    for x in -10..=10 {
      for z in -10..=10 {
        self.chunk(ChunkPos::new(x, z), |_| {});
      }
    }
    info!("done generating terrain");
  }

  pub(super) async fn player_init(&self, player: &Player, conn: &Connection) {
    let mut out = cb::Packet::new(cb::ID::Login);
    out.set_int("entity_id", self.eid());
    out.set_byte("game_mode", 1); // Creative
    out.set_byte("difficulty", 1); // Normal
    if player.ver() < ProtocolVersion::V1_16 {
      out.set_byte("dimension", 0); // Overworld
    }
    out.set_str("level_type", "default".into());
    out.set_byte("max_players", 0); // Ignored
    out.set_bool("reduced_debug_info", false); // Don't reduce debug info

    // 1.13+
    out.set_byte("view_distance", 10); // 10 chunk view distance TODO: Don't hardcode view distance

    // 1.15+
    out.set_byte("hashed_seed", 0);
    out.set_bool("enable_respawn_screen", true);

    // 1.16+
    if player.ver() >= ProtocolVersion::V1_16 {
      out.set_bool("is_hardcore", false);
      out.set_bool("is_flat", false); // Changes the horizon line
      out.set_byte("previous_game_mode", 1);
      out.set_str("world_name", "overworld".into());
      out.set_bool("is_debug", false); // This is not reduced_debug_info, this is for the world being a debug world

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
      out.set_byte_arr("dimension_codec", codec.serialize());
      out.set_byte_arr("dimension", NBT::new("", dimension).serialize());
      out.set_str_arr("world_names", vec!["minecraft:overworld".into()]);
    }

    conn.send(out).await;
    if player.ver() >= ProtocolVersion::V1_13 {
      conn.send(self.get_commands().serialize().await).await;
    }

    for x in -10..=10 {
      for z in -10..=10 {
        conn.send(self.serialize_chunk(ChunkPos::new(x, z), player.ver().block())).await;
      }
    }

    let mut out = cb::Packet::new(cb::ID::Position);
    out.set_double("x", 0.0); // X
    out.set_double("y", 60.0); // Y
    out.set_double("z", 0.0); // Z
    out.set_float("yaw", 0.0); // Yaw
    out.set_float("pitch", 0.0); // Pitch
    out.set_byte("flags", 0); // Flags
    out.set_int("teleport_id", 1234); // TP id
    conn.send(out).await;

    let mut info = PlayerList {
      action: player_list::Action::AddPlayer.into(),
      players: vec![player_list::Player {
        uuid:             Some(player.id().as_proto()),
        name:             player.username().into(),
        properties:       vec![],
        gamemode:         1,
        ping:             300, // TODO: Ping
        has_display_name: false,
        display_name:     "".into(),
      }],
      ..Default::default()
    };
    let mut spawn_packets = vec![];
    for other in self.players().await.iter().in_view(ChunkPos::new(0, 0)).not(player.id()) {
      // Add player to the list of players that other knows about
      let mut out = cb::Packet::new(cb::ID::PlayerInfo);
      out
        .set_other(Other::PlayerList(PlayerList {
          action:  player_list::Action::AddPlayer.into(),
          players: vec![player_list::Player {
            uuid:             Some(player.id().as_proto()),
            name:             player.username().into(),
            properties:       vec![],
            gamemode:         1,
            ping:             300, // TODO: Ping
            has_display_name: false,
            display_name:     "".into(),
          }],
        }))
        .unwrap();
      other.conn().send(out).await;
      // Create a packet that will spawn player for other
      let mut out = cb::Packet::new(cb::ID::NamedEntitySpawn);
      out.set_int("entity_id", player.eid());
      out.set_uuid("player_uuid", player.id());
      let (pos, pitch, yaw) = player.pos_look();
      out.set_double("x", pos.x());
      out.set_double("y", pos.y());
      out.set_double("z", pos.z());
      out.set_float("yaw", yaw);
      out.set_float("pitch", pitch);
      out.set_short("current_item", 0);
      out.set_byte_arr("metadata", player.metadata(other.ver()).serialize());
      other.conn().send(out).await;

      // Add other to the list of players that player knows about
      info.players.push(player_list::Player {
        uuid:             Some(other.id().as_proto()),
        name:             other.username().into(),
        properties:       vec![],
        gamemode:         1,
        ping:             300,
        has_display_name: false,
        display_name:     "".into(),
      });
      // Create a packet that will spawn other for player
      let mut out = cb::Packet::new(cb::ID::NamedEntitySpawn);
      out.set_int("entity_id", other.eid());
      out.set_uuid("player_uuid", other.id());
      let (pos, pitch, yaw) = other.pos_look();
      out.set_double("x", pos.x());
      out.set_double("y", pos.y());
      out.set_double("z", pos.z());
      out.set_float("yaw", yaw);
      out.set_float("pitch", pitch);
      out.set_short("current_item", 0);
      out.set_byte_arr("metadata", other.metadata(player.ver()).serialize());
      spawn_packets.push(out);
    }
    // Need to send the player info before the spawn packets
    let mut out = cb::Packet::new(cb::ID::PlayerInfo);
    out.set_other(Other::PlayerList(info)).unwrap();
    conn.send(out).await;
    for p in spawn_packets {
      conn.send(p).await;
    }

    let mut out = cb::Packet::new(cb::ID::Abilities);
    out.set_byte("flags", 0x04 | 0x08);
    out.set_float("flying_speed", 10.0 * 0.05);
    out.set_float("walking_speed", 0.1);
    conn.send(out).await;
  }
}
