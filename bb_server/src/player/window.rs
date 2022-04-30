use super::Inventory;
use crate::item::Stack;

#[derive(Debug, Clone)]
pub enum Window {
  Generic9x1 {
    inv: Inventory<9>,
  },
  Generic9x2 {
    inv: Inventory<18>,
  },
  Generic9x3 {
    inv: Inventory<27>,
  },
  Generic9x4 {
    inv: Inventory<36>,
  },
  Generic9x5 {
    inv: Inventory<45>,
  },
  Generic9x6 {
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
        Self::Generic9x1 { inv } => inv.$name(idx, $($arg),*),
        Self::Generic9x2 { inv } => inv.$name(idx, $($arg),*),
        Self::Generic9x3 { inv } => inv.$name(idx, $($arg),*),
        Self::Generic9x4 { inv } => inv.$name(idx, $($arg),*),
        Self::Generic9x5 { inv } => inv.$name(idx, $($arg),*),
        Self::Generic9x6 { inv } => inv.$name(idx, $($arg),*),
        Self::Generic3x3 { inv } => inv.$name(idx, $($arg),*),
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
  pub fn ty(&self) -> &'static str {
    match self {
      Self::Generic9x1 { .. } => "minecraft:generic_9x1",
      Self::Generic9x2 { .. } => "minecraft:generic_9x2",
      Self::Generic9x3 { .. } => "minecraft:generic_9x3",
      Self::Generic9x4 { .. } => "minecraft:generic_9x4",
      Self::Generic9x5 { .. } => "minecraft:generic_9x5",
      Self::Generic9x6 { .. } => "minecraft:generic_9x6",
      Self::Generic3x3 { .. } => "minecraft:generic_3x3",
      Self::Anvil { .. } => "minecraft:anvil",
      Self::Beacon { .. } => "minecraft:beacon",
      Self::BlastFurnace { .. } => "minecraft:blast_furnace",
      Self::BrewingStand { .. } => "minecraft:brewing_stand",
      Self::Crafting { .. } => "minecraft:crafting",
      Self::Enchantment { .. } => "minecraft:enchantment",
      Self::Furnace { .. } => "minecraft:furnace",
      Self::Grindstone { .. } => "minecraft:grindstone",
      Self::Hopper { .. } => "minecraft:hopper",
      Self::Lectern { .. } => "minecraft:lectern",
      Self::Loom { .. } => "minecraft:loom",
      Self::Merchant { .. } => "minecraft:merchant",
      Self::ShulkerBox { .. } => "minecraft:shulker_box",
      Self::Smithing { .. } => "minecraft:smithing",
      Self::Smoker { .. } => "minecraft:smoker",
      Self::Cartography { .. } => "minecraft:cartography",
      Self::Stonecutter { .. } => "minecraft:stonecutter",
    }
  }
  pub fn size(&self) -> u32 {
    match self {
      Self::Generic9x1 { .. } => 9,
      Self::Generic9x2 { .. } => 18,
      Self::Generic9x3 { .. } => 27,
      Self::Generic9x4 { .. } => 36,
      Self::Generic9x5 { .. } => 45,
      Self::Generic9x6 { .. } => 54,
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
