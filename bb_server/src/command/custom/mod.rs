use self::{fly::FlyCommand, flyspeed::FlySpeedCommand};

use super::CommandTree;

mod fly;
mod flyspeed;

pub fn add_custom_commands(commands: &CommandTree) {
  FlyCommand::init(commands);
  FlySpeedCommand::init(commands);
}
