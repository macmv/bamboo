use crate::util::Chat;
use bb_macros::Transfer;
use std::{collections::HashMap, num::NonZeroU8};

#[derive(Transfer, Debug, Clone, PartialEq)]
pub struct Item {
  pub id:     i32,
  pub count:  u8,
  // Only exists on 1.8-1.12 clients. 1.13+ clients use NBT for this
  pub damage: i16,
  /// This stores the same data as item NBT, but is version agnostic, and is
  /// converted to NBT on the proxy.
  pub data:   ItemData,
}

#[derive(Transfer, Default, Debug, Clone, PartialEq)]
pub struct ItemData {
  pub display:      ItemDisplay,
  /// A map of latest-version enchantment ids to levels. The level cannot be
  /// zero.
  pub enchantments: Option<HashMap<u32, NonZeroU8>>,
  pub unbreakable:  bool,
}
#[derive(Transfer, Default, Debug, Clone, PartialEq)]
pub struct ItemDisplay {
  /// If `None`, the item will have it's original name. If `Some`, the item will
  /// have the given custom name.
  ///
  /// If no formatting is applied to the name, it will not be italicized, even
  /// though vanilla would normally italicize it. The proxy is responsible for
  /// making sure the correct chat is sent to make the chat not italic.
  pub name: Option<Chat>,
  pub lore: Vec<Chat>,
}

impl Default for Item {
  fn default() -> Self { Item::new(0, 0, 0) }
}

impl Item {
  pub fn new(id: i32, count: u8, damage: i16) -> Self {
    Item { id, count, damage, data: ItemData::default() }
  }

  pub fn id(&self) -> i32 { self.id }
  pub fn count(&self) -> u8 { self.count }
  pub fn data(&self) -> &ItemData { &self.data }
}

impl ItemData {
  pub const fn new() -> Self {
    ItemData { display: ItemDisplay::new(), enchantments: None, unbreakable: false }
  }
  pub fn enchantments_mut(&mut self) -> &mut HashMap<u32, NonZeroU8> {
    self.enchantments.get_or_insert_with(HashMap::new)
  }
}
impl ItemDisplay {
  pub const fn new() -> Self { ItemDisplay { name: None, lore: vec![] } }
}
