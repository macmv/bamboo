use std::sync::Arc;

use bb_common::util::{Chat, GameMode};

use crate::{
  command::{Arg, Command, CommandTree, Parser},
  player::Player,
  world::WorldManager,
};

pub struct GameModeCommand {}

impl GameModeCommand {
  pub fn init(commands: &CommandTree) {
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
}
