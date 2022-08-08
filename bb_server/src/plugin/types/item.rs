use super::{add_from, chat::PChat, wrap};
use crate::{
  item,
  item::{Inventory, Stack, UI},
};
use bb_common::{net::sb::ClickWindow, util::chat};
use bb_server_macros::define_ty;
use panda::{
  parse::token::Span,
  runtime::{RuntimeError, Var},
};
use std::str::FromStr;

wrap!(UI, PUI);
wrap!(ClickWindow, PClickWindow);
wrap!(Inventory<27>, PInventory);
wrap!(Stack, PStack);

#[define_ty(panda_path = "bamboo::item::ClickWindow")]
impl PClickWindow {}

#[define_ty(panda_path = "bamboo::item::Inventory")]
impl PInventory {}

#[define_ty(panda_path = "bamboo::item::Stack")]
impl PStack {
  pub fn new(name: &str) -> Result<Self, RuntimeError> {
    Ok(PStack {
      inner: Stack::new(
        item::Type::from_str(name)
          .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?,
      ),
    })
  }

  pub fn with_amount(&self, amount: u8) -> Self {
    PStack { inner: self.inner.clone().with_amount(amount) }
  }

  pub fn item_name(&self) -> String { self.inner.item().to_str().into() }

  /// Sets the display name of this item stack.
  pub fn set_display_name(&mut self, name: Var) {
    let msg = format!("{}r{}", chat::CODE_SEP, PChat::from_var(name).to_codes());
    self.inner.nbt_mut().get_or_create_compound("display").insert("Name".into(), msg.into());
  }
  /// Sets the lore for this item stack. These are lines of text that show up
  /// below the item name, when hovering over the item in an inventory.
  pub fn set_lore(&mut self, lines: Vec<Var>) {
    self.inner.nbt_mut().get_or_create_compound("display").insert(
      "Lore".into(),
      lines
        .into_iter()
        .map(|msg| format!("{}r{}", chat::CODE_SEP, PChat::from_var(msg).to_codes()))
        .collect::<Vec<String>>()
        .into(),
    );
  }

  /// Sets the item to be unbreakable. If unbreakable is `true`, the item will
  /// not lose durability.
  pub fn set_unbreakable(&mut self, unbreakable: bool) {
    self.inner.nbt_mut().compound_mut().insert("Unbreakable".into(), unbreakable.into());
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
#[define_ty(panda_path = "bamboo::item::UI")]
impl PUI {
  /// Returns the block kind for that string. This will return an error if the
  /// block name is invalid.
  pub fn new(rows: Vec<String>) -> Result<PUI, RuntimeError> {
    Ok(PUI {
      inner: UI::new(rows.iter().map(|v| v.into()).collect())
        .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?,
    })
  }

  pub fn item(&mut self, key: &str, item: &PStack) -> Result<(), RuntimeError> {
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

  pub fn to_inventory(&self) -> Result<PInventory, RuntimeError> {
    let inv = self
      .inner
      .to_inventory()
      .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?;
    Ok(PInventory { inner: inv })
  }
}
