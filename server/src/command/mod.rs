//! This package handles commands. The [`Command`] type can be used to declare
//! a command. The arguments on that command tell both the server and the client
//! how it should parse/autocomplete commands.
//!
//! # Examples
//!
//! ```
//! let mut c = Command::new("fill");
//! c.add_arg("first", Parser::BlockPos); // The string is just the name of the argument. It serves no purpose in parsing.
//! c.add_arg("second", Parser::BlockPos);
//! c.add_arg("block", Parser::BlockState);
//! ```
//!
//! This would create a fill command, which would take three arguments: two
//! block positions, and a block type. The block positions are 3 numbers
//! seperated by spaces. However, they also support relative coordinates: `~10`
//! means 10 blocks up/right/forward of your current position. See the
//! [`Parser`] type for details on the various parsers.
mod enums;
mod parse;

pub use enums::{Arg, Parser};
pub use parse::ParseError;

/// A single command. This can be used to construct an entire command. However,
/// it is also used to represent an argument of a command. When you call
/// [`add_arg`](Self::add_arg) or [`add_lit`](Self::add_lit), these functions
/// will add a new argument to the command. But they will also return a command.
/// This is a seperate struct, and is a reference into the command this was
/// called on. This makes chaining [`add_arg`](Self::add_arg) add arguments
/// one-after-another.
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
  /// Creates a new command. This should be used when you want an entirely new
  /// command (not an argument of another command).
  pub fn new(name: &str) -> Self {
    Self::lit(name.into())
  }
  /// Creates a new literal node. Use [`add_lit`](Self::add_lit) if you want to
  /// add a literal node to the current command.
  fn lit(name: String) -> Self {
    Command { name, ty: NodeType::Literal, children: vec![] }
  }
  /// Creates a new argument node. Use [`add_arg`](Self::add_arg) if you want to
  /// add an argument node to the current command.
  fn arg(name: String, parser: Parser) -> Self {
    Command { name, ty: NodeType::Argument(parser), children: vec![] }
  }
  /// Adds a new literal argument to the command. Unlike
  /// [`add_arg`](Self::add_arg), the name has a purpose in parsing here.
  /// Literal arguments match the exact text of the name in a command. For
  /// example, you might have a command that works like this:
  ///
  /// ```plain
  /// /fill rect ~ ~ ~ ~20 ~ ~20 minecraft:dirt
  /// /fill circle ~ ~ ~ 10.0 minecraft:stone
  /// ```
  ///
  /// And you would implement that command like this:
  ///
  /// ```
  /// let mut c = Command::new("fill");
  /// c.add_lit("rect")
  ///   .add_arg("min", Parser::BlockPos)
  ///   .add_arg("max", Parser::BlockPos)
  ///   .add_arg("block", Parser::BlockState);
  /// c.add_lit("circle")
  ///   .add_arg("pos", Parser::BlockPos)
  ///   .add_arg("radius", Parser::Float { min: Some(0.0), max: None })
  ///   .add_arg("block", Parser::BlockState);
  /// ```
  ///
  /// Note that the `&mut Command` returned by this function is not a refernce
  /// to `self`. It is a new struct, created by this function. It is returned so
  /// that you can chain arguments, and add multiple arguments after each other
  /// easily.
  pub fn add_lit(&mut self, name: &str) -> &mut Command {
    self.children.push(Command::lit(name.into()));
    let index = self.children.len() - 1;
    self.children.get_mut(index).unwrap()
  }
  /// Adds a new argument to this command. Unlike literal arguments, the name
  /// does not matter here. It will only be show to the client when they are
  /// autocompleting the command. Other than that, it is never used. The
  /// important part about this function is the parser. Each parser will match a
  /// certain amount of text. They range from a single float, to an entity
  /// pattern (things like `@a` or `@e[type=Skeleton]`), to a block name
  /// (`minecraft:bedrock`). See the [`Parser`] type for details about every
  /// parser.
  pub fn add_arg(&mut self, name: &str, parser: Parser) -> &mut Command {
    self.children.push(Command::arg(name.into(), parser));
    let index = self.children.len() - 1;
    self.children.get_mut(index).unwrap()
  }
  /// Parses the given text. The given text should be the entire command without
  /// a slash at the start. If anything went wrong during parsing, a ParseError
  /// will be returned. Otherwise, a list of fields will be returned.
  pub fn parse(&self, text: &str) -> Result<Vec<Arg>, ParseError> {
    let (arg, index) = self.parse_arg(text)?;
    if self.children.is_empty() && index < text.len() {
      return Err(ParseError::Trailing(text[index..].into()));
    }
    let mut out = vec![arg];
    let mut errors = vec![];
    for c in &self.children {
      match c.parse(&text[index..]) {
        Ok(v) => {
          out.extend(v);
          break;
        }
        Err(e) => {
          errors.push(e);
        }
      }
    }
    if !self.children.is_empty() && out.len() == 1 {
      Err(ParseError::NoChildren(errors))
    } else {
      Ok(out)
    }
  }
  /// Tries to parse the current argument with the given text. This will ignore
  /// any children that this command may have. If successful, this will return
  /// the argument, and the starting index of the next argument.
  ///
  /// This can be used with top level commands to check if they match some text.
  pub fn parse_arg(&self, text: &str) -> Result<(Arg, usize), ParseError> {
    match &self.ty {
      NodeType::Root => panic!("cannot call matches on root node!"),
      NodeType::Literal => {
        if text.starts_with(&(self.name.clone() + " ")) {
          Ok((Arg::Literal(self.name.clone()), self.name.len() + 1))
        } else {
          Err(ParseError::InvalidLiteral(self.name.clone()))
        }
      }
      NodeType::Argument(p) => p.parse(text),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::block;
  use common::math::Pos;
  use std::collections::HashMap;

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
  }

  #[test]
  fn parse() -> Result<(), ParseError> {
    let mut c = Command::new("fill");
    c.add_arg("min", Parser::BlockPos)
      .add_arg("max", Parser::BlockPos)
      .add_arg("block", Parser::BlockState);
    // let v = match c.parse("fill 20 20 20 10 30 10 minecraft:stone") {
    //   Ok(v) => v,
    //   Err(e) => panic!("{}", e),
    // };
    // assert_eq!(
    //   v,
    //   vec![
    //     Arg::Literal("fill".into()),
    //     Arg::BlockPos(Pos::new(20, 20, 20)),
    //     Arg::BlockPos(Pos::new(10, 30, 10)),
    //     Arg::BlockState(block::Kind::Stone, HashMap::new(), None),
    //   ]
    // );
    Ok(())
  }
}
