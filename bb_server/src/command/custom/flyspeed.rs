use crate::command::{Command, CommandTree, Parser};

pub struct FlySpeedCommand {}

impl FlySpeedCommand {
  pub fn init(commands: &CommandTree) {
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
}
