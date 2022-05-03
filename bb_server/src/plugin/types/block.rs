use super::{add_from, wrap};
use crate::block;
use bb_plugin_macros::define_ty;
use panda::{parse::token::Span, runtime::RuntimeError};
use std::str::FromStr;

wrap!(block::Kind, PBlockKind);
wrap!(block::Type, PBlockType);

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
#[define_ty(panda_path = "bamboo::block::Kind")]
impl PBlockKind {
  /// Returns the block kind for that string. This will return an error if the
  /// block name is invalid.
  pub fn from_s(name: &str) -> Result<PBlockKind, RuntimeError> {
    Ok(PBlockKind {
      inner: block::Kind::from_str(name).map_err(|_| {
        RuntimeError::custom(format!("invalid block name '{}'", name), Span::call_site())
      })?,
    })
  }
  /// Returns the name of this block. This is the same name passed to `from_s`.
  pub fn to_s(&self) -> String { self.inner.to_str().to_string() }
}

#[define_ty(panda_path = "bamboo::block::Type")]
impl PBlockType {
  /// Returns the name of this block. This is the same name passed to `from_s`.
  pub fn to_s(&self) -> String { self.inner.to_string() }
}
