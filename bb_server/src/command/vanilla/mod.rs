use self::{fill::FillCommand, gamemode::GameModeCommand, say::SayCommand, summon::SummonCommand};

use super::CommandTree;

pub fn add_vanilla_commands(commands: &CommandTree) {
  add_fill_command(commands);
  add_gamemode_command(commands);
  add_say_command(commands);
  add_summon_command(commands);
}

fn add_fill_command(commands: &CommandTree) {
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
  commands.add(c, |world, _, args| {
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
}

fn add_gamemode_command(commands: &CommandTree) {
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
          if let Some(player) = target.as_player() {
            player.set_game_mode(gm);
            player.send_message(Chat::new(format!(
              "Set {}'s game mode to {}",
              player.username(),
              gm
            )));
          }
        }
      }
    } else if let Some(player) = runner {
      player.set_game_mode(gm);
      player.send_message(Chat::new(format!("Set own game mode to {}", gm)));
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
    commands.add(c, handle_gamemode);
  }

  let add_specific_game_mode = |name: &'static str, gm: GameMode| {
    let mut c = Command::new(name);
    c.add_arg_opt("target", Parser::Entity { single: false, only_players: true });
    commands.add(c, move |wm, runner, args| {
      if let Some(arg) = args.get(1) {
        for world in wm.worlds().iter() {
          for target in arg.entity().iter(&world.entities(), runner) {
            if let Some(player) = target.as_player() {
              player.set_game_mode(gm);
              player.send_message(Chat::new(format!(
                "Set {}'s game mode to {}",
                player.username(),
                gm
              )));
            }
          }
        }
      } else if let Some(player) = runner {
        player.set_game_mode(gm);
        player.send_message(Chat::new(format!("Set own game mode to {}", gm)));
      } else {
        // TODO: Send error saying they need to specify a target
      }
    });
  };

  add_specific_game_mode("gms", GameMode::Survival);
  add_specific_game_mode("gmc", GameMode::Creative);
  add_specific_game_mode("gma", GameMode::Adventure);
  add_specific_game_mode("gmsp", GameMode::Spectator);
}

fn add_say_command(commands: &CommandTree) {
  let mut c = Command::new("say");
  c.add_arg("text", Parser::String(StringType::Greedy));
  commands.add(c, |world, player, args| {
    world.broadcast(format!("[Server] {}", args[1].str()).as_str());
  });
}

fn add_summon_command(commands: &CommandTree) {
  let mut c = Command::new("summon");
  c.add_arg("entity", Parser::EntitySummon);
  commands.add(c, |_, player, args| {
    // args[0] is `summon`
    let ty = args[1].entity_summon();
    if let Some(p) = player {
      let eid = p.world().summon(ty, p.pos());
      info!("eid of mob: {}", eid);
      p.send_message(Chat::new(format!("summoned {ty:?}")));
    }
  });
}
