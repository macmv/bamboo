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

pub fn add_command(cmd: &Command) {
  unsafe {
    let ffi = cmd.to_ffi();
    log::info!("{ffi:#?}");
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
      name:       bb_ffi::CPtr::new(self.name.as_ptr()),
      name_len:   self.name.len() as u32,
      node_type:  match self.ty {
        NodeType::Literal => 0,
        NodeType::Argument(_) => 1,
      },
      parser:     bb_ffi::CPtr::new(match &self.ty {
        NodeType::Literal => 0 as _,
        NodeType::Argument(parser) => parser.as_ptr(),
      }),
      parser_len: match &self.ty {
        NodeType::Literal => 0,
        NodeType::Argument(parser) => parser.len() as u32,
      },
      optional:   self.optional as u8,
      children:   bb_ffi::CList::new(self.children.iter().map(|c| c.to_ffi()).collect()),
    }
  }
}
