use super::World;
use crate::command::{Command, Parser, StringType};
use common::{math::ChunkPos, util::Chat};

impl World {
  pub async fn init(&self) {
    let mut c = Command::new("say");
    c.add_arg("text", Parser::String(StringType::Greedy));
    self
      .get_commands()
      .add(
        c,
        Box::new(|world, _| {
          Box::new(async move {
            world.broadcast(&Chat::new("[Server] big announce")).await;
          })
        }),
      )
      .await;

    let mut c = Command::new("fill");
    c.add_lit("rect")
      .add_arg("min", Parser::BlockPos)
      .add_arg("max", Parser::BlockPos)
      .add_arg("block", Parser::BlockState);
    c.add_lit("circle")
      .add_arg("center", Parser::BlockPos)
      .add_arg("radius", Parser::Float { min: Some(0.0), max: None })
      .add_arg("block", Parser::BlockState);
    self
      .get_commands()
      .add(
        c,
        Box::new(|world, _| {
          Box::new(async move {
            world.broadcast(&Chat::new("called /fill")).await;
          })
        }),
      )
      .await;

    info!("generating terrain...");
    for x in -10..=10 {
      for z in -10..=10 {
        self.chunk(ChunkPos::new(x, z), |_| {});
      }
    }
    info!("done generating terrain");
  }
}
