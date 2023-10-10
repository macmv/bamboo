use super::Bamboo;
use crate::block;
use bb_server_macros::define_ty;
use panda::{
  parse::token::Span,
  runtime::{Result, RuntimeError, Var, VarSend},
};
use std::str::FromStr;

/// A block kind. This is how you get/set blocks in the world.
///
/// You should use this by importing `bamboo::block`. This will make your
/// code much easier to read. For example:
///
/// ```
/// use sugarlang::block
///
/// fn main() {
///   world.set_kind(Pos::new(0, 60, 0), block::Kind::from_s("stone"))
/// }
/// ```
///
/// If you instead use `Kind` on its own, it is much less clear that this is
/// a block kind.
#[define_ty]
impl PBlockKind {
  info! {
    wrap: block::Kind,

    panda: {
      path: "bamboo::block::Kind",
    },
    python: {
      class: "BlockKind",
    },
  }

  /// Returns the block kind for that string. This will return an error if the
  /// block name is invalid.
  pub fn from_s(name: &str) -> Result<PBlockKind> {
    Ok(PBlockKind {
      inner: block::Kind::from_str(name).map_err(|_| {
        RuntimeError::custom(format!("invalid block name '{name}'"), Span::call_site())
      })?,
    })
  }
  /// Returns the name of this block. This is the same name passed to `from_s`.
  pub fn to_s(&self) -> String { self.inner.to_str().to_string() }
}

#[define_ty(panda_path = "bamboo::block::Type")]
impl PBlockType {
  info! {
    wrap: block::TypeStore,

    panda: {
      path: "bamboo::block::Type",
    },
    python: {
      class: "BlockType",
    },
  }

  /// Returns the name of this block. This is the same name passed to `from_s`.
  pub fn to_s(&self) -> String { self.inner.to_string() }
  /// Returns the kind of this block.
  pub fn kind(&self) -> PBlockKind { self.inner.kind().into() }

  pub fn with(&mut self, prop: &str, value: &str) -> Self {
    self.inner.set_prop(prop, value);
    self.clone()
  }
}

#[define_ty]
impl PBlockData {
  info! {
    wrap: block::Data,

    panda: {
      path: "bamboo::block::Data",
    },
    python: {
      class: "BlockData",
    },
  }

  /// Returns the default type of the given block.
  pub fn default_type(&self) -> PBlockType {
    PBlockType::from(self.inner.default_type().to_store())
  }
}

#[derive(Clone, Debug)]
pub struct Behaviors {
  pub bb: Bamboo,
  pub wm: Arc<WorldManager>,
}

struct PluginBehavior {
  bb:       Bamboo,
  behavior: VarSend,
}

use crate::{
  block::{Block, BlockDrops, Data, TypeOrStore},
  event::EventFlow,
  player::{BlockClick, Player},
  world::{World, WorldManager},
};
use bb_common::math::Pos;
use std::sync::Arc;

impl block::Behavior for PluginBehavior {
  fn place<'a>(&self, data: &'a Data, pos: Pos, click: BlockClick) -> TypeOrStore<'a> {
    info!("placing in custom behavior");

    let idx = self.bb.idx;
    let plugins = self.bb.wm.plugins().plugins.lock();
    let plugin = &plugins[idx];
    let mut imp = plugin.imp.lock();
    let pd = imp.panda().unwrap();

    let mut env = pd.lock_env();

    let var: Var = self.behavior.clone().into();
    let path = var.ty().to_path().join(&panda::path!(place));

    match env.call(
      &path,
      Span::call_site(),
      Some(var),
      vec![
        PBlockData::from(data.clone()).into(),
        super::util::PPos::from(pos).into(),
        super::player::PBlockClick {
          player: click.player.clone(),
          dir:    click.dir,
          block:  super::player::Block {
            world: click.block.world.clone(),
            pos:   click.block.pos,
            ty:    click.block.ty.to_store(),
          },
          face:   click.face,
          cursor: click.cursor,
        }
        .into(),
      ],
    ) {
      Ok(v) => {
        let b = v.strct(Span::call_site()).unwrap().as_builtin(Span::call_site()).unwrap();
        let ty = b.as_any().downcast_ref::<PBlockType>().unwrap();
        return ty.inner.clone().into();
      }
      Err(e) => pd.print_err(e),
    }

    let _ = (pos, click);
    data.default_type().into()
  }
  fn update_place(&self, world: &Arc<World>, block: Block) { let _ = (world, block); }
  fn update(&self, world: &Arc<World>, block: Block, old: Block, new: Block) {
    let _ = (world, block, old, new);
  }
  fn interact(&self, block: Block, player: &Arc<Player>) -> EventFlow {
    let _ = (block, player);
    EventFlow::Continue
  }
  fn drops(&self, block: Block) -> BlockDrops {
    let _ = block;
    BlockDrops::Normal
  }
}

#[define_ty]
impl PBlockBehaviors {
  info! {
    wrap: Behaviors,

    panda: {
      path: "bamboo::block::Behaviors",
    },
    python: {
      class: "BlockBehaviors",
    },
  }

  pub fn register(&self, name: &str, behavior: Var) -> Result<Var> {
    let kind = block::Kind::from_str(name).map_err(|_| {
      RuntimeError::custom(format!("invalid block name '{name}'"), Span::call_site())
    })?;
    self
      .inner
      .wm
      .block_behaviors_mut()
      .behaviors
      .set(kind, Box::new(PluginBehavior { bb: self.inner.bb.clone(), behavior: behavior.into() }));
    info!("set custom behavior for {kind:?}");
    Ok(Var::None)
  }
}
