use crate::{
  block,
  player::Player,
  world::{World, WorldManager},
};
use common::{math::Pos, util::Chat};
use rutie::{AnyObject, Fixnum, Module, NilClass, Object, RString, VerifiedObject};
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

class!(WorldRb);
wrappable_struct!(Arc<World>, WorldW, WORLD);

impl WorldRb {
  pub fn new(world: Arc<World>) -> Self {
    Module::from_existing("Sugarcane").get_nested_class("World").wrap_data(world, &*WORLD)
  }
  pub fn to_world(&self) -> &Arc<World> {
    &self.get_data(&*WORLD)
  }
}

methods!(
  WorldRb,
  rself,
  fn world_set_block(pos: PosRb, kind: Fixnum) -> NilClass {
    info!("setting block");
    let w = rself.to_world();
    tokio::task::block_in_place(|| {
      tokio::runtime::Handle::current().block_on(async move {
        w.set_kind(pos.unwrap().to_pos(), block::Kind::from_u32(kind.unwrap().to_u64() as u32))
          .await
          .unwrap();
      })
    });

    NilClass::new()
  },
);

class!(PlayerRb);
wrappable_struct!(Arc<Player>, PlayerW, PLAYER);

impl PlayerRb {
  pub fn new(player: Arc<Player>) -> Self {
    Module::from_existing("Sugarcane").get_nested_class("Player").wrap_data(player, &*PLAYER)
  }
  pub fn to_player(&self) -> &Arc<Player> {
    &self.get_data(&*PLAYER)
  }
}

methods!(
  PlayerRb,
  rself,
  fn player_username() -> RString {
    RString::new_utf8(rself.to_player().username())
  },
  fn player_world() -> WorldRb {
    WorldRb::new(rself.to_player().clone_world())
  },
);

class!(PosRb);
wrappable_struct!(Pos, PosW, POS);

impl PosRb {
  pub fn new(pos: Pos) -> Self {
    Module::from_existing("Sugarcane").get_nested_class("Pos").wrap_data(pos, &*POS)
  }
  pub fn to_pos(&self) -> Pos {
    *self.get_data(&*POS)
  }
}

impl VerifiedObject for PosRb {
  fn is_correct_type<T: Object>(object: &T) -> bool {
    object.class() == Module::from_existing("Sugarcane").get_nested_class("Pos")
  }

  fn error_message() -> &'static str {
    "Error converting to PosRb"
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
    Fixnum::new(rself.to_pos().x().into())
  },
  fn pos_y() -> Fixnum {
    Fixnum::new(rself.to_pos().y().into())
  },
  fn pos_z() -> Fixnum {
    Fixnum::new(rself.to_pos().z().into())
  },
  fn pos_to_s() -> RString {
    RString::new_utf8(&format!("{}", rself.to_pos()))
  },
);

// macro_rules! variadic_methods {
//   (
//     $rself_class: ty,
//     $rself_name: ident,
//     $(
//       fn $method_name: ident
//       ($args_name: ident: &[AnyObject]) -> $return_type: ident $body: block
//       $(,)?
//     )*
//   ) => {
//     $(
//       #[allow(unused_mut, improper_ctypes_definitions)]
//       pub extern fn $method_name(
//         argc: Argc,
//         argv: *const AnyObject,
//         mut $rself_name: $rself_class,
//       ) -> $return_type {
//         let $args_name = rutie::util::parse_arguments(argc, argv);
//
//         $body
//       }
//     )*
//   };
// }

module!(SugarcaneMod);
methods!(
  SugarcaneMod,
  rself,
  fn trace(msg: RString) -> NilClass {
    trace!("{}", msg.unwrap().to_str());
    NilClass::new()
  },
  fn debug(msg: RString) -> NilClass {
    debug!("{}", msg.unwrap().to_str());
    NilClass::new()
  },
  fn info(msg: RString) -> NilClass {
    info!("{}", msg.unwrap().to_str());
    NilClass::new()
  },
  fn warn(msg: RString) -> NilClass {
    warn!("{}", msg.unwrap().to_str());
    NilClass::new()
  },
  fn error(msg: RString) -> NilClass {
    error!("{}", msg.unwrap().to_str());
    NilClass::new()
  }
);

/// Creates the Sugarcane ruby module. This file handles all wrapper
/// classes/methods for all types that are defined in Ruby.
pub fn create_module() {
  Module::new("Sugarcane").define(|c| {
    c.define_nested_class("Sugarcane", None).define(|c| {
      c.def("broadcast", sc_broadcast);
    });
    c.define_nested_class("World", None).define(|c| {
      c.def("set_block", world_set_block);
    });
    c.define_nested_class("Player", None).define(|c| {
      c.def("username", player_username);
      c.def("world", player_world);
    });
    c.define_nested_class("Pos", None).define(|c| {
      c.def_self("new", pos_new);

      c.def("x", pos_x);
      c.def("y", pos_y);
      c.def("z", pos_z);

      c.def("to_s", pos_to_s);
    });
    c.define_nested_module("Block").define(|c| {
      for (i, name) in block::names().iter().enumerate() {
        c.const_set(&name.to_string().to_ascii_uppercase(), &Fixnum::new(i as i64));
      }
    });
    c.def_self("trace", trace);
    c.def_self("debug", debug);
    c.def_self("info", info);
    c.def_self("warn", warn);
    c.def_self("error", error);
  });
}
