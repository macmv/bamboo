use crate::player::Player;
use parking_lot::{lock_api::RawMutex, Mutex};
use std::collections::HashMap;

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
  pub fn new(carg: bb_ffi::CArg) -> Self {
    if let Some(s) = carg.into_literal() {
      Arg::Lit(s.into_string())
    } else {
      todo!()
    }
  }
  pub fn lit(&self) -> &str {
    match self {
      Self::Lit(s) => s.as_str(),
      _ => panic!("not a literal: {self:?}"),
    }
  }
}

static CALLBACKS: Mutex<Option<HashMap<String, Box<dyn Fn(Option<Player>, Vec<Arg>) + Send>>>> =
  Mutex::const_new(parking_lot::RawMutex::INIT, None);
pub fn add_command(cmd: &Command, cb: impl Fn(Option<Player>, Vec<Arg>) + Send + 'static) {
  {
    let mut cbs = CALLBACKS.lock();
    if cbs.is_none() {
      *cbs = Some(HashMap::new());
    }
    cbs.as_mut().unwrap().insert(cmd.name.clone(), Box::new(cb));
  }
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

#[no_mangle]
extern "C" fn on_command(player: *mut bb_ffi::CUUID, args: *mut bb_ffi::CList<bb_ffi::CArg>) {
  unsafe {
    let player = if player.is_null() { None } else { Some(Box::from_raw(player)) };
    let args = Box::from_raw(args);
    let args: Vec<_> = args.into_vec().into_iter().map(|carg| Arg::new(carg)).collect();
    let name = args[0].lit();
    let cb = CALLBACKS.lock();
    if let Some(cb) = cb.as_ref() {
      cb[name](player.map(|id| Player::new(*id)), args);
    }
  }
}
