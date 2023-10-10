use self::{fill::FillCommand, gamemode::GameModeCommand, say::SayCommand, summon::SummonCommand};

use super::CommandTree;

mod fill;
mod gamemode;
mod say;
mod summon;

pub fn add_vanilla_commands(commands: &CommandTree) {
  FillCommand::init(commands);
  GameModeCommand::init(commands);
  SayCommand::init(commands);
  SummonCommand::init(commands);
}
