use super::{Command, CommandTree, Parser};

pub fn add_custom_commands(commands: &CommandTree) {
  add_fly_command(commands);
  add_flyspeed_command(commands);
}

fn add_flyspeed_command(commands: &CommandTree) {
  let mut c = Command::new("flyspeed");
  c.add_arg("multiplier", Parser::Float { min: None, max: None });
  commands.add(c, |_, player, args| {
    // args[0] is `flyspeed`
    let v = args[1].float();
    if let Some(p) = player {
      p.set_flyspeed(v);
    }
  });
}

fn add_fly_command(commands: &CommandTree) {
  let c = Command::new("fly");
  commands.add(c, |_, player, _| {
    if let Some(p) = player {
      p.set_flying_allowed(!p.flying_allowed());
    }
  });
}
