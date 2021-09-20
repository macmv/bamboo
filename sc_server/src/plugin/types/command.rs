use super::{add_from, wrap};
use crate::command::{Arg, Command, Parser};
use std::sync::{Arc, Mutex};
use sugarlang::{
  define_ty,
  runtime::{Callback, VarData},
};

wrap!(Arc<Mutex<Command>>, SlCommand, callback: Option<Callback>, idx: Vec<usize>);
wrap!(Arg, SlArg);

impl SlCommand {
  fn command<'a>(&self, inner: &'a mut Command) -> &'a mut Command {
    let mut c = inner;
    for idx in &self.idx {
      c = c.get_child(*idx).unwrap();
    }
    c
  }
}

/// A command. This is how to setup the arguments for a custom commands that
/// users can run.
#[define_ty(path = "sugarcane::command::Command")]
impl SlCommand {
  /// Creates a new command. The callback must be a function, which takes 3
  /// arguments. See the example for details.
  ///
  /// # Example
  ///
  /// ```
  /// fn main() {
  ///   c = Command::new("setblock", handle_setblock)
  /// }
  ///
  /// fn handle_setblock(sc, player, args) {
  ///   sc.info("ran setblock!")
  /// }
  /// ```
  pub fn new(name: &str, callback: Callback) -> SlCommand {
    SlCommand {
      inner:    Arc::new(Mutex::new(Command::new(name))),
      callback: Some(callback),
      idx:      vec![],
    }
  }
  /// Adds a new block position argument to the command.
  ///
  /// This will be parsed as three numbers in a row. If you use a `~` before the
  /// block coordinates, they will be parsed as relative coordinates. So if you
  /// are standing at X: 50, then `~10` will be converted into X: 60.
  pub fn add_arg_block_pos(&mut self, name: &str) -> SlCommand {
    let mut lock = self.inner.lock().unwrap();
    self.command(&mut lock).add_arg(name, Parser::BlockPos);
    let mut idx = self.idx.clone();
    idx.push(self.command(&mut lock).children_len() - 1);
    SlCommand { inner: self.inner.clone(), callback: None, idx }
  }
  /// Adds a literal to the command.
  ///
  /// This is a special type of argument. It matches the exact text of the name.
  /// This should only be used if you want to expect a keyword.
  ///
  /// # Example
  ///
  /// ```
  /// c = Command::new("fill", handle_fill)
  /// c.add_arg_lit("rect")
  ///   .add_arg_block_pos("min")
  ///   .add_arg_block_pos("max")
  /// c.add_arg_lit("circle")
  ///   .add_arg_block_pos("center")
  ///   .add_arg_float("radius")
  /// ```
  ///
  /// This will parse the following commands:
  /// ```
  /// /fill rect ~ ~ ~ ~ ~ ~
  /// /fill rect 5 5 5 20 20 20
  ///
  /// /fill circle ~ ~ ~ 5
  /// /fill circle 6 7 8 20
  /// ```
  ///
  /// As you can see, this should only be used when you have a keyword you need
  /// the user to type in. See `add_arg_word` if you are expecting a single
  /// word.
  pub fn add_lit(&mut self, name: &str) -> SlCommand {
    let mut lock = self.inner.lock().unwrap();
    self.command(&mut lock).add_lit(name);
    let mut idx = self.idx.clone();
    idx.push(self.command(&mut lock).children_len() - 1);
    SlCommand { inner: self.inner.clone(), callback: None, idx }
  }
}

/// A command argument. This is how you read back the arguments that a user
/// passed to your command.
#[define_ty(path = "sugarcane::command::Arg")]
impl SlArg {
  /// If this argument is a literal, then this returns the value of that
  /// literal. Otherwise, this will return an error.
  pub fn lit(&self) -> String {
    self.inner.lit().to_string()
  }
}
