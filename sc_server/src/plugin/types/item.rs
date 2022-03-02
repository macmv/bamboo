use super::{add_from, wrap};
use crate::{
  item,
  item::{Stack, UI},
};
use std::str::FromStr;
use sugarlang::{define_ty, parse::token::Span, runtime::RuntimeError};

wrap!(UI, SlUI);

/// An inventory UI.
///
/// You should use this by importing `sugarcane::block`. This will make your
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
#[define_ty(path = "sugarcane::item::UI")]
impl SlUI {
  /// Returns the block kind for that string. This will return an error if the
  /// block name is invalid.
  pub fn new(rows: Vec<&str>) -> Result<SlUI, RuntimeError> {
    Ok(SlUI {
      inner: UI::new(rows.iter().map(|&v| v.into()).collect())
        .map_err(|e| RuntimeError::Custom(e.to_string(), Span::default()))?,
    })
  }

  pub fn item(&mut self, key: &str, item: &str) -> Result<(), RuntimeError> {
    let mut iter = key.chars();
    let key = match iter.next() {
      Some(v) => v,
      None => {
        return Err(RuntimeError::Custom(
          "Cannot use empty string as item key".into(),
          Span::default(),
        ))
      }
    };
    if iter.next().is_some() {
      return Err(RuntimeError::Custom(
        "Cannot use multiple character string as item key".into(),
        Span::default(),
      ));
    }
    self.inner.item(
      key,
      Stack::new(
        item::Type::from_str(item)
          .map_err(|e| RuntimeError::Custom(e.to_string(), Span::default()))?,
      ),
    );
    Ok(())
  }
}
