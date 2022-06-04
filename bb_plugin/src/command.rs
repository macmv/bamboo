use crate::player::Player;

pub struct Command {
  name:     String,
  ty:       NodeType,
  children: Vec<Command>,
  optional: bool,
}
#[derive(Debug, Clone)]
enum NodeType {
  Literal,
  Argument(String),
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Arg {
  Lit(String),
}

impl Arg {
  pub fn lit(&self) -> &str {
    match self {
      Self::Lit(s) => s.as_str(),
      _ => panic!("not a literal: {self:?}"),
    }
  }
}

pub fn add_command(cmd: &Command, cb: impl Fn(Option<Player>, Vec<Arg>)) {
  unsafe {
    let ffi = cmd.to_ffi();
    bb_ffi::bb_add_command(&ffi);
  }
}

impl Command {
  pub fn new(name: impl Into<String>) -> Self {
    Command {
      name:     name.into(),
      ty:       NodeType::Literal,
      children: vec![],
      optional: false,
    }
  }
  pub fn add_arg(&mut self, name: impl Into<String>, parser: impl Into<String>) -> &mut Command {
    self.children.push(Command {
      name:     name.into(),
      ty:       NodeType::Argument(parser.into()),
      children: vec![],
      optional: false,
    });
    self.children.last_mut().unwrap()
  }
  pub fn add_lit(&mut self, name: impl Into<String>) -> &mut Command {
    self.children.push(Command {
      name:     name.into(),
      ty:       NodeType::Literal,
      children: vec![],
      optional: false,
    });
    self.children.last_mut().unwrap()
  }

  /// # Safety
  /// - `self` is essentially borrowed for the entire lifetime of the returned
  ///   command. This command points to data in `self` which cannot be changed.
  pub(crate) unsafe fn to_ffi(&self) -> bb_ffi::CCommand {
    bb_ffi::CCommand {
      name:      bb_ffi::CStr::new(self.name.clone()),
      node_type: match self.ty {
        NodeType::Literal => 0,
        NodeType::Argument(_) => 1,
      },
      parser:    match &self.ty {
        NodeType::Literal => bb_ffi::CStr::new(String::new()),
        NodeType::Argument(parser) => bb_ffi::CStr::new(parser.clone()),
      },
      optional:  bb_ffi::CBool::new(self.optional),
      children:  bb_ffi::CList::new(self.children.iter().map(|c| c.to_ffi()).collect()),
    }
  }
}
