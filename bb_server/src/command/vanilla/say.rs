use crate::command::{Command, CommandTree, Parser, StringType};

pub struct SayCommand {}

impl SayCommand {
  pub fn init(commands: &CommandTree) {
    let mut c = Command::new("say");
    c.add_arg("text", Parser::String(StringType::Greedy));
    commands.add(c, |world, player, args| {
      world.broadcast(format!("[Server] {}", args[1].str()).as_str());
    });
  }
}
