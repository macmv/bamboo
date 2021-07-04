use super::World;
use crate::command::{Command, Parser, StringType};
use common::math::ChunkPos;

impl World {
  pub fn init(&self) {
    let mut c = Command::new("say");
    c.add_arg("text", Parser::String(StringType::Greedy));
    self.get_commands().add(c);

    info!("generating terrain...");
    for x in -10..=10 {
      for z in -10..=10 {
        self.chunk(ChunkPos::new(x, z), |_| {});
      }
    }
    info!("done generating terrain");
  }
}
