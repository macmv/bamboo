use crate::command::{Command, CommandTree, Parser};

pub struct FillCommand {}

impl FillCommand {
  pub fn init(commands: &CommandTree) {
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
}
