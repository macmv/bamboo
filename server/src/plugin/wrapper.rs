use crate::world::WorldManager;
use common::{math::Pos, util::Chat};
use rutie::{types::Value, AnyObject, Fixnum, Module, NilClass, Object, RString, VerifiedObject};
use std::{fmt, sync::Arc};

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

class!(PosRb);
wrappable_struct!(Pos, PosWrapper, POS_WRAPPER);

impl PosRb {
  pub fn new(pos: Pos) -> Self {
    Module::from_existing("Sugarcane").get_nested_class("Pos").wrap_data(pos, &*POS_WRAPPER)
  }
}

methods!(
  PosRb,
  rtself,
  fn pos_new(x: Fixnum, y: Fixnum, z: Fixnum) -> AnyObject {
    let pos = Pos::new(x.unwrap().to_i32(), y.unwrap().to_i32(), z.unwrap().to_i32());
    Module::from_existing("Sugarcane").get_nested_class("Pos").wrap_data(pos, &*POS_WRAPPER)
  },
  fn pos_x() -> Fixnum {
    info!("running ruby x method");
    Fixnum::new(rtself.get_data(&*POS_WRAPPER).x().into())
  },
);

/// Creates the Sugarcane ruby module. This file handles all wrapper
/// classes/methods for all types that are defined in Ruby.
pub fn create_module() -> Module {
  let mut sc = Module::new("Sugarcane");
  sc.define(|c| {
    c.define_nested_class("Pos", None).define(|c| {
      c.def_self("new", pos_new);

      c.def("x", pos_x);
    });
    c.define_nested_class("Sugarcane", None).define(|c| {
      c.define_method("broadcast", sc_broadcast);
    });
  });
  sc
}
