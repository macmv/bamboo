use super::{add_from, block::SlBlockKind, util::SlPos};
use crate::world::World;
use std::{fmt, sync::Arc};
use sugarlang::define_ty;

#[derive(Clone)]
pub struct SlWorld {
  pub(super) inner: Arc<World>,
}

add_from!(Arc<World>, SlWorld);

impl fmt::Debug for SlWorld {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("SlWorld").finish()
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
  pub fn set_kind(&self, pos: &SlPos, kind: &SlBlockKind) {
    let w = self.inner.clone();
    let p = pos.inner;
    let k = kind.inner;
    tokio::spawn(async move {
      w.set_kind(p, k).await.unwrap();
    });
  }
  /// Fills a rectangle of blocks in the world. This will return an error if the
  /// min or max are outside of the world.
  ///
  /// This function will do everything you want when filling blocks.. It will
  /// update the blocks stored in the world, and send block updates to all
  /// clients in render distance.
  pub fn fill_rect_kind(&self, min: &SlPos, max: &SlPos, kind: &SlBlockKind) {
    let w = self.inner.clone();
    let min = min.inner;
    let max = max.inner;
    let k = kind.inner;
    tokio::spawn(async move {
      w.fill_rect_kind(min, max, k).await.unwrap();
    });
  }
}
