use super::{add_from, block::SlBlockKind, util::SlPos};
use crate::world::World;
use sc_common::math::Pos;
use std::{fmt, sync::Arc};
use sugarlang::{
  define_ty,
  parse::token::Span,
  runtime::{RuntimeError, VarData},
};

#[derive(Clone)]
pub struct SlWorld {
  pub(super) inner: Arc<World>,
}

add_from!(Arc<World>, SlWorld);

impl fmt::Debug for SlWorld {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.debug_struct("SlWorld").finish() }
}

impl SlWorld {
  pub fn check_pos(&self, pos: Pos) -> Result<Pos, RuntimeError> {
    self.inner.check_pos(pos).map_err(|p| {
      RuntimeError::custom(format!("invalid position {}: {}", p.pos, p.msg), Span::default())
    })
  }
}

/// A Minecraft world. This stores all of the information about blocks,
/// entities, and players in this world.
#[define_ty(path = "sugarcane::world::World")]
impl SlWorld {
  /// Sets a single block in the world. This will return an error if the block
  /// is outside of the world.
  ///
  /// If you need to set multiple blocks at all, you should always use
  /// `fill_kind` instead. It is faster in every situation except for single
  /// blocks (where it is the same speed).
  ///
  /// This function will do everything you want in a block place. It will update
  /// the blocks stored in the world, and send block updates to all clients in
  /// render distance.
  pub fn set_kind(&self, pos: &SlPos, kind: &SlBlockKind) -> Result<(), RuntimeError> {
    self.check_pos(pos.inner)?;
    self.inner.set_kind(pos.inner, kind.inner).unwrap();
    Ok(())
  }
  /// Fills a rectangle of blocks in the world. This will return an error if the
  /// min or max are outside of the world.
  ///
  /// This function will do everything you want when filling blocks.. It will
  /// update the blocks stored in the world, and send block updates to all
  /// clients in render distance.
  pub fn fill_rect_kind(
    &self,
    min: &SlPos,
    max: &SlPos,
    kind: &SlBlockKind,
  ) -> Result<(), RuntimeError> {
    self.check_pos(min.inner)?;
    self.check_pos(max.inner)?;
    self.inner.fill_rect_kind(min.inner, max.inner, kind.inner).unwrap();
    Ok(())
  }
}
