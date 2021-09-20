use super::{add_from, wrap};
use crate::block;
use sugarlang::define_ty;

wrap!(block::Kind, SlBlockKind);

/// A block kind. This is how you get/set blocks in the world.
#[define_ty(path = "sugarcane::block::BlockKind")]
impl SlBlockKind {
  pub fn to_s(&self) -> String {
    format!("{:?}", self.inner)
  }
}
