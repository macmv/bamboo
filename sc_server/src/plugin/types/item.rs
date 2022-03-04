use super::{add_from, wrap};
use crate::{
  item,
  item::{Inventory, Stack, UI},
};
use sc_common::net::sb::ClickWindow;
use std::str::FromStr;
use sugarlang::{define_ty, parse::token::Span, runtime::RuntimeError};

wrap!(UI, SlUI);
wrap!(ClickWindow, SlClickWindow);
wrap!(Inventory, SlInventory);
wrap!(Stack, SlStack);

#[define_ty(path = "sugarcane::item::ClickWindow")]
impl SlClickWindow {}

#[define_ty(path = "sugarcane::item::Inventory")]
impl SlInventory {}

#[define_ty(path = "sugarcane::item::Stack")]
impl SlStack {
  pub fn new(name: &str) -> Result<Self, RuntimeError> {
    Ok(SlStack {
      inner: Stack::new(
        item::Type::from_str(name)
          .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?,
      ),
    })
  }
}

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
        .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?,
    })
  }

  pub fn item(&mut self, key: &str, item: &SlStack) -> Result<(), RuntimeError> {
    let mut iter = key.chars();
    let key = match iter.next() {
      Some(v) => v,
      None => {
        return Err(RuntimeError::Custom(
          "Cannot use empty string as item key".into(),
          Span::call_site(),
        ))
      }
    };
    if iter.next().is_some() {
      return Err(RuntimeError::Custom(
        "Cannot use multiple character string as item key".into(),
        Span::call_site(),
      ));
    }
    self.inner.item(key, item.inner.clone());
    Ok(())
  }

  pub fn to_inventory(&self) -> Result<SlInventory, RuntimeError> {
    let inv = self
      .inner
      .to_inventory()
      .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?;
    Ok(SlInventory { inner: inv })
  }
}
