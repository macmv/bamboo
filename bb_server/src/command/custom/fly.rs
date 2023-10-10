use crate::command::{Command, CommandTree};

pub struct FlyCommand {}

impl FlyCommand {
  pub fn init(commands: &CommandTree) {
    let c = Command::new("fly");
    commands.add(c, |_, player, _| {
      if let Some(p) = player {
        p.set_flying_allowed(!p.flying_allowed());
      }
    });
  }
}
