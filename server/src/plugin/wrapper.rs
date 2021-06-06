use crate::world::WorldManager;
use common::util::Chat;
use rutie::{types::Value, Module, NilClass, Object, RString};
use std::sync::Arc;

#[repr(C)]
#[derive(Clone)]
pub struct SugarcaneRb {
  value: Value,
  wm:    Option<Arc<WorldManager>>,
}

impl SugarcaneRb {
  pub fn new(value: Value, wm: Arc<WorldManager>) -> Self {
    SugarcaneRb { value, wm: Some(wm) }
  }
}

impl From<Value> for SugarcaneRb {
  fn from(value: Value) -> Self {
    SugarcaneRb { value, wm: None }
  }
}

impl Object for SugarcaneRb {
  #[inline]
  fn value(&self) -> Value {
    self.value
  }
}

methods!(
  SugarcaneRb,
  rtself,
  fn sc_broadcast(v: RString) -> NilClass {
    let msg = Chat::new(v.unwrap().to_string());
    let wm = rtself.wm.unwrap();
    info!("Broadcasting?");
    tokio::task::block_in_place(|| {
      tokio::runtime::Handle::current().block_on(async move {
        wm.broadcast(&msg).await;
      })
    });

    NilClass::new()
  },
);

/// Creates the Sugarcane ruby module. This file handles all wrapper
/// classes/methods for all types that are defined in Ruby.
pub fn create_module() -> Module {
  let mut sc = Module::new("Sugarcane");
  sc.define(|c| {
    // c.define_nested_class("Pos", None).define(|c| {
    //   c.def_self("new", pos_new);
    //   c.attr_accessor("x");
    //   c.attr_accessor("y");
    //   c.attr_accessor("z");
    // });
    c.define_nested_class("Sugarcane", None).define(|c| {
      c.define_method("broadcast", sc_broadcast);
    });
  });
}
