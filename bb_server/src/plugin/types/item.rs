use super::chat::PChat;
use crate::{
  item,
  item::{Inventory, Stack, UI},
};
use bb_common::net::sb::ClickWindow;
use bb_server_macros::define_ty;
use panda::{
  parse::token::Span,
  runtime::{RuntimeError, Var},
};
use std::str::FromStr;

#[define_ty]
impl PClickWindow {
  info! {
    wrap: ClickWindow,

    panda: {
      path: "bamboo::item::ClickWindow",
    },
  }
}

#[define_ty]
impl PInventory {
  info! {
    wrap: Inventory<27>,

    panda: {
      path: "bamboo::item::Inventory",
    },
    python: {
      class: "Inventory",
    },
  }
}

#[define_ty]
impl PStack {
  info! {
    wrap: Stack,

    panda: {
      path: "bamboo::item::Stack",
    },
  }

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
    self.inner.data_mut().display.name = Some(PChat::from_var(name));
  }
  /// Sets the lore for this item stack. These are lines of text that show up
  /// below the item name, when hovering over the item in an inventory.
  pub fn set_lore(&mut self, lines: Vec<Var>) {
    self.inner.data_mut().display.lore =
      lines.into_iter().map(|msg| PChat::from_var(msg)).collect::<Vec<bb_common::util::Chat>>();
  }

  /// Sets the item to be unbreakable. If unbreakable is `true`, the item will
  /// not lose durability.
  pub fn set_unbreakable(&mut self, unbreakable: bool) {
    self.inner.data_mut().unbreakable = unbreakable;
  }
  /// Sets the given enchantment to the given level for this stack. If set to 0,
  /// the enchantment will be removed.
  pub fn set_enchantment(&mut self, enchantment: &str, level: u8) -> Result<(), RuntimeError> {
    let enchantment = crate::enchantment::Type::from_str(enchantment)
      .map_err(|e| RuntimeError::Custom(e.to_string(), Span::call_site()))?;
    let enchantments = self.inner.data_mut().enchantments_mut();
    if let Some(level) = std::num::NonZeroU8::new(level) {
      enchantments.insert(enchantment.id(), level);
    } else {
      enchantments.remove(&enchantment.id());
    }
    Ok(())
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
#[define_ty]
impl PUI {
  info! {
    wrap: UI,

    panda: {
      path: "bamboo::item::UI",
    },
  }

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
