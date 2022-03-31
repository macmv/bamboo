use super::{add_from, wrap};
use crate::{
  item,
  item::{Inventory, Stack, UI},
};
use bb_common::net::sb::ClickWindow;
use panda::{define_ty, parse::token::Span, runtime::RuntimeError};
use std::str::FromStr;

wrap!(UI, PdUI);
wrap!(ClickWindow, PdClickWindow);
wrap!(Inventory, PdInventory);
wrap!(Stack, PdStack);

#[define_ty(path = "bamboo::item::ClickWindow")]
impl PdClickWindow {}

#[define_ty(path = "bamboo::item::Inventory")]
impl PdInventory {}

#[define_ty(path = "bamboo::item::Stack")]
impl PdStack {
  pub fn new(name: &str) -> Result<Self, RuntimeError> {
    Ok(PdStack {
      inner: Stack::new(
        item::Type::from_str(name)
          .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?,
      ),
    })
  }

  pub fn with_amount(&self, amount: u8) -> Self {
    PdStack { inner: self.inner.clone().with_amount(amount) }
  }
}

/// An inventory UI.
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
#[define_ty(path = "bamboo::item::UI")]
impl PdUI {
  /// Returns the block kind for that string. This will return an error if the
  /// block name is invalid.
  pub fn new(rows: Vec<String>) -> Result<PdUI, RuntimeError> {
    Ok(PdUI {
      inner: UI::new(rows.iter().map(|v| v.into()).collect())
        .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?,
    })
  }

  pub fn item(&mut self, key: &str, item: &PdStack) -> Result<(), RuntimeError> {
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

  pub fn to_inventory(&self) -> Result<PdInventory, RuntimeError> {
    let inv = self
      .inner
      .to_inventory()
      .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?;
    Ok(PdInventory { inner: inv })
  }
}
