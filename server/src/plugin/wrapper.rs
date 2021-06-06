use crate::world::WorldManager;
use common::{math::Pos, util::Chat};
use rutie::{AnyObject, Fixnum, Module, NilClass, Object, RString};
use std::sync::Arc;

class!(SugarcaneRb);
wrappable_struct!(Arc<WorldManager>, WorldManagerWrapper, WM);

impl SugarcaneRb {
  pub fn new(wm: Arc<WorldManager>) -> Self {
    Module::from_existing("Sugarcane").get_nested_class("Sugarcane").wrap_data(wm, &*WM)
  }
}

methods!(
  SugarcaneRb,
  rself,
  fn sc_broadcast(v: RString) -> NilClass {
    let wm = rself.get_data(&*WM);
    let msg = Chat::new(v.unwrap().to_string());
    tokio::task::block_in_place(|| {
      tokio::runtime::Handle::current().block_on(async move {
        wm.broadcast(&msg).await;
      })
    });

    NilClass::new()
  },
);

class!(PosRb);
wrappable_struct!(Pos, PosW, POS);

impl PosRb {
  pub fn new(pos: Pos) -> Self {
    Module::from_existing("Sugarcane").get_nested_class("Pos").wrap_data(pos, &*POS)
  }
}

methods!(
  PosRb,
  rself,
  fn pos_new(x: Fixnum, y: Fixnum, z: Fixnum) -> AnyObject {
    PosRb::new(Pos::new(x.unwrap().to_i32(), y.unwrap().to_i32(), z.unwrap().to_i32()))
      .value()
      .into()
  },
  fn pos_x() -> Fixnum {
    Fixnum::new(rself.get_data(&*POS).x().into())
  },
  fn pos_y() -> Fixnum {
    Fixnum::new(rself.get_data(&*POS).y().into())
  },
  fn pos_z() -> Fixnum {
    Fixnum::new(rself.get_data(&*POS).z().into())
  },
  fn pos_to_s() -> RString {
    RString::new_utf8(&format!("{}", rself.get_data(&*POS)))
  },
);

/// Creates the Sugarcane ruby module. This file handles all wrapper
/// classes/methods for all types that are defined in Ruby.
pub fn create_module() {
  Module::new("Sugarcane").define(|c| {
    c.define_nested_class("Pos", None).define(|c| {
      c.def_self("new", pos_new);

      c.def("x", pos_x);
      c.def("y", pos_y);
      c.def("z", pos_z);

      c.def("to_s", pos_to_s);
    });
    c.define_nested_class("Sugarcane", None).define(|c| {
      c.define_method("broadcast", sc_broadcast);
    });
  });
}
