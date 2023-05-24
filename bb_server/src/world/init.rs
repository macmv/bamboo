use super::{World, WorldManager};
use crate::{
  command::{Arg, Command, Parser, StringType},
  entity,
  player::Player,
};
use bb_common::{
  math::ChunkPos,
  net::cb,
  util::{Buffer, Chat, GameMode, JoinInfo, JoinMode, SwitchMode},
  version::ProtocolVersion,
};
use std::sync::Arc;

impl World {
  pub fn init(self: &Arc<World>) {
    if self.config().vanilla.enabled {
      self.load_from_disk(&std::path::PathBuf::new().join(&self.config().vanilla.path)).unwrap();
    }

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
    fn handle_gamemode(wm: &Arc<WorldManager>, runner: Option<&Arc<Player>>, args: Vec<Arg>) {
      let gm = match &args[1] {
        Arg::Literal(lit) => match lit.as_str() {
          "survival" | "s" => GameMode::Survival,
          "creative" | "c" => GameMode::Creative,
          "adventure" | "a" => GameMode::Adventure,
          "spectator" | "sp" => GameMode::Spectator,
          _ => unreachable!(),
        },
        Arg::Int(num) => GameMode::from_id(*num as u8),
        _ => unreachable!(),
      };
      if let Some(arg) = args.get(2) {
        for world in wm.worlds().iter() {
          for target in arg.entity().iter(&world.entities(), runner) {
            if let Some(p) = target.as_player() {
              p.set_game_mode(gm)
            }
          }
        }
      } else if let Some(player) = runner {
        player.set_game_mode(gm);
      } else {
        // TODO: Send error saying they need to specify a target
      }
    }
    for name in ["gamemode", "gm"] {
      let mut c = Command::new(name);
      c.add_lit("survival")
        .add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      c.add_lit("creative")
        .add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      c.add_lit("adventure")
        .add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      c.add_lit("spectator")
        .add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      c.add_lit("s").add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      c.add_lit("c").add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      c.add_lit("a").add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      c.add_lit("sp").add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      c.add_arg("mode", Parser::Int { min: Some(0), max: Some(3) });
      self.commands().add(c, handle_gamemode);
    }

    let add_specific_game_mode = |name: &'static str, gm: GameMode| {
      let mut c = Command::new(name);
      c.add_arg_opt("target", Parser::Entity { single: false, only_players: true });
      self.commands().add(c, move |wm, runner, args| {
        if let Some(arg) = args.get(1) {
          for world in wm.worlds().iter() {
            for target in arg.entity().iter(&world.entities(), runner) {
              if let Some(p) = target.as_player() {
                p.set_game_mode(gm)
              }
            }
          }
        } else if let Some(player) = runner {
          player.set_game_mode(gm);
        } else {
          // TODO: Send error saying they need to specify a target
        }
      });
    };

    add_specific_game_mode("gms", GameMode::Survival);
    add_specific_game_mode("gmc", GameMode::Creative);
    add_specific_game_mode("gma", GameMode::Adventure);
    add_specific_game_mode("gmsp", GameMode::Spectator);

    let c = Command::new("fly");
    self.commands().add(c, |_, player, _| {
      if let Some(p) = player {
        p.set_flying_allowed(!p.flying_allowed());
      }
    });
    let mut c = Command::new("flyspeed");
    c.add_arg("multiplier", Parser::Float { min: None, max: None });
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
        p.send_message(Chat::new(format!("summoned {ty:?}")));
      }
    });

    let mut c = Command::new("join");
    c.add_arg("addr", Parser::String(StringType::Word));
    self.commands().add(c, |_, player, args| {
      if let Some(p) = player {
        let addr = match args[1].str().parse::<std::net::SocketAddr>() {
          Ok(a) => a,
          Err(e) => {
            p.send_message(Chat::new(e.to_string()));
            return;
          }
        };
        p.switch_to(SwitchMode::Loading, vec![addr]);
      }
    });

    info!("generating terrain...");
    /*
    let chunks = Mutex::new(vec![]);
    let loaded = AtomicU32::new(0);
    let view_distance = self.wm.config().get::<_, i32>("view-distance");
    let total = ((view_distance * 2 + 1) * (view_distance * 2 + 1)) as f64;
    (-view_distance..=view_distance).into_par_iter().for_each(|x| {
      for z in -view_distance..=view_distance {
        let pos = ChunkPos::new(x, z);
        let c = self.pre_generate_chunk(pos);
        chunks.lock().push((pos, c));

        let num_loaded = loaded.fetch_add(1, Ordering::SeqCst);
        let old_progress = (f64::from(num_loaded) / total) * 100.0;
        let new_progress = (f64::from(num_loaded + 1) / total) * 100.0;
        const INC: f64 = 10.0;
        // We want 10% increments, so if the fetch_add just put us over a 10% increment,
        // we log it.
        if (old_progress / INC) as i32 != (new_progress / INC) as i32 {
          info!("{:0.2}%", new_progress);
        }
      }
    });
    self.store_chunks_no_overwrite(chunks.into_inner());
    // Keep spawn chunks always loaded
    for x in -view_distance..=view_distance {
      for z in -view_distance..=view_distance {
        let pos = ChunkPos::new(x, z);
        self.inc_view(pos);
      }
    }
    */
    info!("done generating terrain");
  }

  pub(super) fn player_init(self: &Arc<Self>, player: &Player, info: JoinInfo) {
    match info.mode {
      JoinMode::New => {
        let out = cb::packet::JoinGame {
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
          // view_distance_v1_14: Some(10), // 10 chunk view distance TODO: Per player view
          // distance
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
          // is_debug_v1_16:           Some(false), /* This is not reduced_debug_info, this is for
          // the
          //                                         * world being a debug world */
          // dimension_codec_v1_16:    Some(codec.serialize()),
          // dimension_v1_16:          Some("".into()),
          // dimension_v1_16_2:        Some(NBT::new("", dimension).serialize()),
          // world_names_v1_16:        Some(world_names.into_inner()),

          // Every player thinks they are EID 1, and no player is
          // actually EID 1. This makes switching servers very easy.
          eid:                   1,
          hardcore_mode:         false,
          game_mode:             player.game_mode(),
          dimension:             0, // Overworld
          level_type:            "default".into(),
          difficulty:            1, // Normal
          view_distance:         player.view_distance() as u16,
          reduced_debug_info:    false,
          enable_respawn_screen: true,
          world_height:          self.height,
          world_min_y:           self.min_y,
        };

        player.send(out);
      }
      JoinMode::Switch(SwitchMode::Loading) => {
        player.send(cb::packet::Respawn {
          difficulty: 1,
          dimension:  1,
          game_mode:  player.game_mode(),
          level_type: "default".into(),
          reset_meta: true,
        });
        player.send(cb::packet::Respawn {
          difficulty: 1,
          dimension:  0,
          game_mode:  player.game_mode(),
          level_type: "default".into(),
          reset_meta: true,
        });
        let pos = player.pos().chunk();
        for x in -(player.view_distance() as i32)..=player.view_distance() as i32 {
          for z in -(player.view_distance() as i32)..=player.view_distance() as i32 {
            player.send(cb::packet::UnloadChunk { pos: pos + ChunkPos::new(x, z) });
          }
        }
      }
      JoinMode::Switch(SwitchMode::Seemless) => {}
    }

    if player.ver() >= ProtocolVersion::V1_13 {
      // TODO: Fix tags for older versions.
      if player.ver() >= ProtocolVersion::V1_18_2 {
        player.send(self.world_manager().tags().serialize());
      }
      player.send(self.commands().serialize());
    }

    player.send(cb::packet::EntityStatus {
      // Players think they are EID 1.
      eid:    1,
      // Set op permission to level 4
      // Note that 24 is op permission 0, 25 is op permission 1, etc.
      status: 28,
    });

    let d = player.view_distance() as i32;
    for x in -d..=d {
      for z in -d..=d {
        let pos = ChunkPos::new(x, z);
        self.inc_view(pos);
        player.send_chunk(pos, || self.serialize_chunk(pos).into());
      }
    }

    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    buf.write_str("Bamboo");
    player.send(cb::packet::PluginMessage { channel: "minecraft:brand".into(), data });

    let pos = player.pos();
    player.send(cb::packet::SetPosLook {
      pos,
      yaw: 0.0,
      pitch: 0.0,
      flags: 0,
      teleport_id: 1234,
      should_dismount: true,
    });

    let my_info = cb::PlayerListAdd {
      id:           player.id(),
      name:         player.username().clone(),
      game_mode:    GameMode::Creative,
      ping:         50,
      display_name: player.tab_name().clone().map(|c| c.to_json()),
    };
    let my_info_packet =
      cb::packet::PlayerList { action: cb::PlayerListAction::Add(vec![my_info.clone()]) };

    // We need to add my info into the packet going to me, because minecraft is
    // weird.
    let mut info = vec![my_info];
    for other in self.players().iter().not(player.id()) {
      // Lets `other` know that I exist
      other.send(my_info_packet.clone());

      // Add `other` to the list of players that I know about
      info.push(cb::PlayerListAdd {
        id:           other.id(),
        name:         other.username().clone(),
        game_mode:    GameMode::Creative,
        ping:         50,
        display_name: other.tab_name().clone().map(|c| c.to_json()),
      });
    }
    player.send(cb::packet::PlayerList { action: cb::PlayerListAction::Add(info) });

    for other in self.entities().iter() {
      if !player.in_view(other.pos().block().chunk()) {
        continue;
      }
      if let Some(other) = other.as_player() {
        // We don't want either packet if this is the same player.
        if other.id() == player.id() {
          continue;
        }
        // Create a packet that will spawn me for `other`
        let (pos, pitch, yaw) = player.pos_look();
        other.send(cb::packet::SpawnPlayer {
          eid: player.eid(),
          id: player.id(),
          ty: entity::Type::Player.id(),
          pos,
          yaw: yaw as i8,
          pitch: pitch as i8,
          meta: player.metadata(),
        });

        // Create a packet that will spawn `other` for me
        let (pos, pitch, yaw) = other.pos_look();
        player.send(cb::packet::SpawnPlayer {
          eid: other.eid(),
          id: other.id(),
          ty: entity::Type::Player.id(),
          pos,
          yaw: yaw as i8,
          pitch: pitch as i8,
          meta: other.metadata(),
        });
      }
      if other.as_entity().is_some() {
        // Create a packet that will spawn `other` for me
        self.send_entity_spawn(player, &other);
      }
    }

    for (_, team) in self.wm.teams().iter() {
      team.lock().send_join(player);
    }
  }
}
