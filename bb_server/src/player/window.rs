use super::Inventory;
use crate::item::Stack;

#[derive(Debug, Clone)]
pub enum Window {
  Generic1x9 {
    inv: Inventory<9>,
  },
  Generic2x9 {
    inv: Inventory<18>,
  },
  Generic3x9 {
    inv: Inventory<27>,
  },
  Generic4x9 {
    inv: Inventory<36>,
  },
  Generic5x9 {
    inv: Inventory<45>,
  },
  Generic6x9 {
    inv: Inventory<54>,
  },
  Generic3x3 {
    inv: Inventory<9>,
  },
  Anvil {
    tool:    Inventory<1>,
    enchant: Inventory<1>,
    output:  Inventory<1>,
  },
  Beacon {
    inv: Inventory<1>,
  },
  BlastFurnace {
    input:  Inventory<1>,
    fuel:   Inventory<1>,
    output: Inventory<1>,
  },
  BrewingStand {
    bottles:    Inventory<3>,
    ingredient: Inventory<1>,
    fuel:       Inventory<1>,
  },
  Crafting {
    grid:   Inventory<9>,
    output: Inventory<1>,
  },
  Enchantment {
    book:  Inventory<1>,
    lapis: Inventory<1>,
  },
  Furnace {
    input:  Inventory<1>,
    fuel:   Inventory<1>,
    output: Inventory<1>,
  },
  Grindstone {
    inputs: Inventory<2>,
    output: Inventory<1>,
  },
  Hopper {
    inv: Inventory<5>,
  },
  Lectern {
    book: Inventory<1>,
  },
  Loom {
    banner:  Inventory<1>,
    dye:     Inventory<1>,
    pattern: Inventory<1>,
    output:  Inventory<1>,
  },
  Merchant {
    inv: Inventory<1>,
  },
  ShulkerBox {
    inv: Inventory<27>,
  },
  Smithing {
    input:   Inventory<1>,
    upgrade: Inventory<1>,
    output:  Inventory<1>,
  },
  Smoker {
    input:  Inventory<1>,
    fuel:   Inventory<1>,
    output: Inventory<1>,
  },
  Cartography {
    map:    Inventory<1>,
    paper:  Inventory<1>,
    output: Inventory<1>,
  },
  Stonecutter {
    input:  Inventory<1>,
    output: Inventory<1>,
  },
}
macro_rules! for_all {
  ( $self:ty, $name:ident (idx: u32 $(, $arg:ident: $ty:ty)*) $( -> $ret:ty )?, $default:expr) => {
    pub fn $name(self: $self, idx: u32, $($arg: $ty),*) $( -> $ret )? {
      match self {
        Self::Generic3x9 { inv } => inv.$name(idx, $($arg),*),
        Self::Crafting { grid, output } => match idx {
          0..=8 => grid.$name(idx, $($arg),*),
          9..=9 => output.$name(idx, $($arg),*),
          _ => $default,
        },
        _ => todo!(),
      }
    }
  };
}

pub struct ItemsIter<'a> {
  win:   &'a Window,
  index: u32,
}

impl<'a> Iterator for ItemsIter<'a> {
  type Item = &'a Stack;

  fn next(&mut self) -> Option<Self::Item> {
    self.win.get(self.index).map(|it| {
      self.index += 1;
      it
    })
  }
}

impl Window {
  for_all!(&mut Self, set(idx: u32, stack: Stack), {});
  for_all!(&Self, get(idx: u32) -> Option<&Stack>, None);
  for_all!(&mut Self, get_mut(idx: u32) -> Option<&mut Stack>, None);
  pub fn items(&self) -> ItemsIter<'_> { ItemsIter { win: self, index: 0 } }
  pub fn add(&mut self, _stack: &Stack) -> u8 { todo!() }
  pub fn ty(&self) -> u8 {
    match self {
      Self::Generic1x9 { .. } => 0,
      Self::Generic2x9 { .. } => 1,
      Self::Generic3x9 { .. } => 2,
      Self::Generic4x9 { .. } => 3,
      Self::Generic5x9 { .. } => 4,
      Self::Generic6x9 { .. } => 5,
      Self::Generic3x3 { .. } => 6,
      Self::Anvil { .. } => 7,
      Self::Beacon { .. } => 8,
      Self::BlastFurnace { .. } => 9,
      Self::BrewingStand { .. } => 10,
      Self::Crafting { .. } => 11,
      Self::Enchantment { .. } => 12,
      Self::Furnace { .. } => 13,
      Self::Grindstone { .. } => 14,
      Self::Hopper { .. } => 15,
      Self::Lectern { .. } => 16,
      Self::Loom { .. } => 17,
      Self::Merchant { .. } => 18,
      Self::ShulkerBox { .. } => 19,
      Self::Smithing { .. } => 20,
      Self::Smoker { .. } => 21,
      Self::Cartography { .. } => 22,
      Self::Stonecutter { .. } => 23,
    }
  }
  pub fn size(&self) -> u32 {
    match self {
      Self::Generic1x9 { .. } => 9,
      Self::Generic2x9 { .. } => 18,
      Self::Generic3x9 { .. } => 27,
      Self::Generic4x9 { .. } => 36,
      Self::Generic5x9 { .. } => 45,
      Self::Generic6x9 { .. } => 54,
      Self::Generic3x3 { .. } => 9,
      Self::Anvil { .. } => 3,
      Self::Beacon { .. } => 1,
      Self::BlastFurnace { .. } => 3,
      Self::BrewingStand { .. } => 5,
      Self::Crafting { .. } => 10,
      Self::Enchantment { .. } => 2,
      Self::Furnace { .. } => 3,
      Self::Grindstone { .. } => 3,
      Self::Hopper { .. } => 5,
      Self::Lectern { .. } => 1,
      Self::Loom { .. } => 4,
      Self::Merchant { .. } => 1,
      Self::ShulkerBox { .. } => 27,
      Self::Smithing { .. } => 3,
      Self::Smoker { .. } => 3,
      Self::Cartography { .. } => 3,
      Self::Stonecutter { .. } => 2,
    }
  }
}
