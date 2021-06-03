mod parser;

pub use parser::Parser;

#[derive(Debug, Clone)]
pub struct Command {
  name:     String,
  ty:       NodeType,
  children: Vec<Command>,
}
#[derive(Debug, Clone)]
enum NodeType {
  Root,
  Literal,
  Argument(Parser),
}

impl Command {
  pub fn new(name: &str) -> Self {
    Self::lit(name.into())
  }
  fn lit(name: String) -> Self {
    Command { name, ty: NodeType::Literal, children: vec![] }
  }
  fn arg(name: String, parser: Parser) -> Self {
    Command { name, ty: NodeType::Argument(parser), children: vec![] }
  }
  pub fn add_lit(&mut self, name: &str) -> &mut Command {
    self.children.push(Command::lit(name.into()));
    let index = self.children.len() - 1;
    self.children.get_mut(index).unwrap()
  }
  pub fn add_arg(&mut self, name: &str, parser: Parser) -> &mut Command {
    self.children.push(Command::arg(name.into(), parser));
    let index = self.children.len() - 1;
    self.children.get_mut(index).unwrap()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn construction() {
    let mut c = Command::new("fill");
    c.add_lit("rect")
      .add_arg("min", Parser::BlockPos)
      .add_arg("max", Parser::BlockPos)
      .add_arg("block", Parser::BlockState);
    c.add_lit("circle")
      .add_arg("pos", Parser::BlockPos)
      .add_arg("radius", Parser::Float { min: Some(0.0), max: None })
      .add_arg("block", Parser::BlockState);
    c.add_lit("sphere")
      .add_arg("pos", Parser::BlockPos)
      .add_arg("radius", Parser::Float { min: Some(0.0), max: None })
      .add_arg("block", Parser::BlockState);
    dbg!(c);
    panic!();
  }
}
