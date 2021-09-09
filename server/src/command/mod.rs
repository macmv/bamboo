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
mod reader;
mod serialize;

pub use enums::{Arg, Parser, StringType};
use parse::{ChildError, Span};
pub use parse::{ErrorKind, ParseError, Tokenizer};

use crate::{player::Player, world::WorldManager};
use common::util::chat::{Chat, Color};
use reader::CommandReader;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};
use tokio::sync::Mutex;

type Handler = Box<
  dyn Fn(Arc<WorldManager>, Vec<Arg>) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
    + Send
    + Sync,
>;

/// All of the commands on a server. This is a table of all the commands that
/// the clients can run. It handles serializing these commands to packets, and
/// callbacks for when a command is run. It also delegates all of the command
/// parsing that needs to be done for callbacks to work.
pub struct CommandTree {
  commands: Mutex<HashMap<String, (Command, Handler)>>,
}

impl CommandTree {
  /// Creates an empty command tree. This is called whenever a `World` is
  /// created.
  pub fn new() -> CommandTree {
    CommandTree { commands: Mutex::new(HashMap::new()) }
  }
  /// Adds a new command to the tree. Any new players that join will be able to
  /// execute this command. This will also update the `/help` output, and
  /// include the command syntax/description.
  pub async fn add<F, Fut>(&self, c: Command, handler: F)
  where
    F: (Fn(Arc<WorldManager>, Vec<Arg>) -> Fut) + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
  {
    self.commands.lock().await.insert(
      c.name().into(),
      (c, Box::new(move |world, command| Box::pin((handler)(world, command)))),
    );
  }
  /// Called whenever a command should be executed. This can also be used to act
  /// like a player sent a command, even if they didn't. The text passed in
  /// should not contain a `/` at the start.
  pub async fn execute(&self, world: Arc<WorldManager>, player: Arc<Player>, text: &str) {
    let mut reader = CommandReader::new(text);
    let commands = self.commands.lock().await;
    let (command, handler) = match &commands.get(&reader.word(StringType::Word).unwrap()) {
      Some(v) => v,
      None => {
        let mut msg = Chat::empty();
        msg.add(""); // Makes the default color white
        msg.add("Unknown command: ").color(Color::Red);
        msg.add(text);
        player.send_message(&msg).await;
        return;
      }
    };
    let args = match command.parse(text) {
      Ok(v) => v,
      Err(e) => {
        player.send_message(&e.to_chat(text)).await;
        return;
      }
    };
    handler(world, args).await;
  }
}

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
  pub fn new<N: Into<String>>(name: N) -> Self {
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
    self.parse_inner(&mut Tokenizer::new(text))
  }

  fn parse_inner(&self, tokens: &mut Tokenizer) -> Result<Vec<Arg>, ParseError> {
    let arg = self.parse_arg(tokens)?;
    // if self.children.is_empty() && index < text.len() {
    //   return Err(ParseError::Trailing(text[index..].into()));
    // }
    let mut out = vec![arg];
    let mut errors = vec![];
    let mut err_span = None;
    for c in &self.children {
      match c.parse_inner(&mut tokens.clone()) {
        Ok(v) => {
          out.extend(v);
          break;
        }
        Err(e) => {
          // If all the errors have the same span, use that, otherwise just use the
          // token's position.
          if let Some(span) = err_span {
            if span != e.pos() {
              err_span = Some(Span::single(tokens.pos()));
            }
          } else {
            err_span = Some(e.pos());
          }
          match &c.ty {
            NodeType::Root => unreachable!(),
            NodeType::Literal => errors.push(ChildError::Expected(c.name.clone())),
            NodeType::Argument(p) => errors.push(ChildError::Invalid(p.clone())),
          }
        }
      }
    }
    if !self.children.is_empty() && out.len() == 1 {
      Err(ParseError::new(err_span.unwrap(), ErrorKind::NoChildren(errors)))
    } else {
      Ok(out)
    }
  }
  /// Tries to parse the current argument with the given text. This will ignore
  /// any children that this command may have. If successful, this will return
  /// the argument, and the starting index of the next argument.
  ///
  /// This can be used with top level commands to check if they match some text.
  fn parse_arg(&self, tokens: &mut Tokenizer) -> Result<Arg, ParseError> {
    match &self.ty {
      NodeType::Root => panic!("cannot call matches on root node!"),
      NodeType::Literal => {
        let w = tokens.read_spaced_word()?;
        if w == self.name.as_ref() {
          Ok(Arg::Literal(self.name.clone()))
        } else {
          Err(ParseError::new(w.pos(), ErrorKind::Invalid))
        }
      }
      NodeType::Argument(p) => p.parse(tokens),
    }
  }

  /// Returns the name of the command. This does not contain a slash at the
  /// start.
  pub fn name(&self) -> &str {
    &self.name
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
    let v = match c.parse("fill 20 20 20 10 30 10 stone") {
      Ok(v) => v,
      Err(e) => panic!("{}", e),
    };
    assert_eq!(
      v,
      vec![
        Arg::Literal("fill".into()),
        Arg::BlockPos(Pos::new(20, 20, 20)),
        Arg::BlockPos(Pos::new(10, 30, 10)),
        Arg::BlockState(block::Kind::Stone, HashMap::new(), None),
      ]
    );
    Ok(())
  }
}
