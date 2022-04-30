use crate::item::{SharedInventory, Stack};

#[derive(Debug, Clone)]
pub enum Window {
  Generic9x1 {
    inv: SharedInventory<9>,
  },
  Generic9x2 {
    inv: SharedInventory<18>,
  },
  Generic9x3 {
    inv: SharedInventory<27>,
  },
  Generic9x4 {
    inv: SharedInventory<36>,
  },
  Generic9x5 {
    inv: SharedInventory<45>,
  },
  Generic9x6 {
    inv: SharedInventory<54>,
  },
  Generic3x3 {
    inv: SharedInventory<9>,
  },
  Anvil {
    tool:    SharedInventory<1>,
    enchant: SharedInventory<1>,
    output:  SharedInventory<1>,
  },
  Beacon {
    inv: SharedInventory<1>,
  },
  BlastFurnace {
    input:  SharedInventory<1>,
    fuel:   SharedInventory<1>,
    output: SharedInventory<1>,
  },
  BrewingStand {
    bottles:    SharedInventory<3>,
    ingredient: SharedInventory<1>,
    fuel:       SharedInventory<1>,
  },
  Crafting {
    output: SharedInventory<1>,
    grid:   SharedInventory<9>,
  },
  Enchantment {
    book:  SharedInventory<1>,
    lapis: SharedInventory<1>,
  },
  Furnace {
    input:  SharedInventory<1>,
    fuel:   SharedInventory<1>,
    output: SharedInventory<1>,
  },
  Grindstone {
    inputs: SharedInventory<2>,
    output: SharedInventory<1>,
  },
  Hopper {
    inv: SharedInventory<5>,
  },
  Lectern {
    book: SharedInventory<1>,
  },
  Loom {
    banner:  SharedInventory<1>,
    dye:     SharedInventory<1>,
    pattern: SharedInventory<1>,
    output:  SharedInventory<1>,
  },
  Merchant {
    inv: SharedInventory<1>,
  },
  ShulkerBox {
    inv: SharedInventory<27>,
  },
  Smithing {
    input:   SharedInventory<1>,
    upgrade: SharedInventory<1>,
    output:  SharedInventory<1>,
  },
  Smoker {
    input:  SharedInventory<1>,
    fuel:   SharedInventory<1>,
    output: SharedInventory<1>,
  },
  Cartography {
    map:    SharedInventory<1>,
    paper:  SharedInventory<1>,
    output: SharedInventory<1>,
  },
  Stonecutter {
    input:  SharedInventory<1>,
    output: SharedInventory<1>,
  },
}
macro_rules! for_all {
  ( $self:ty, $name:ident $call:ident (idx: u32, $stack:ty)) => {
    pub fn $name<R>(self: $self, idx: u32, f: impl FnOnce($stack) -> R) -> Option<R> {
      match self {
        Self::Generic9x1 { inv } => inv.lock().$call(idx).map(|s| f(s)),
        Self::Generic9x2 { inv } => inv.lock().$call(idx).map(|s| f(s)),
        Self::Generic9x3 { inv } => inv.lock().$call(idx).map(|s| f(s)),
        Self::Generic9x4 { inv } => inv.lock().$call(idx).map(|s| f(s)),
        Self::Generic9x5 { inv } => inv.lock().$call(idx).map(|s| f(s)),
        Self::Generic9x6 { inv } => inv.lock().$call(idx).map(|s| f(s)),
        Self::Generic3x3 { inv } => inv.lock().$call(idx).map(|s| f(s)),
        Self::Crafting { output, grid } => match idx {
          0..=0 => output.lock().$call(idx).map(|s| f(s)),
          1..=9 => grid.lock().$call(idx - 1).map(|s| f(s)),
          _ => None,
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

impl Iterator for ItemsIter<'_> {
  type Item = Stack;

  fn next(&mut self) -> Option<Self::Item> {
    self.win.get(self.index).map(|it| {
      self.index += 1;
      it
    })
  }
}

impl Window {
  pub fn get(&self, index: u32) -> Option<Stack> { self.access(index, |s| s.clone()) }
  pub fn set(&mut self, index: u32, stack: Stack) { self.access_mut(index, move |s| *s = stack); }
  for_all!(&Self, access get(idx: u32, &Stack));
  for_all!(&mut Self, access_mut get_mut(idx: u32, &mut Stack));
  pub fn items(&self) -> ItemsIter<'_> { ItemsIter { win: self, index: 0 } }
  pub fn add(&mut self, stack: &Stack) -> u8 {
    match self {
      Self::Generic9x1 { inv } => inv.lock().add(stack),
      Self::Generic9x2 { inv } => inv.lock().add(stack),
      Self::Generic9x3 { inv } => inv.lock().add(stack),
      Self::Generic9x4 { inv } => inv.lock().add(stack),
      Self::Generic9x5 { inv } => inv.lock().add(stack),
      Self::Generic9x6 { inv } => inv.lock().add(stack),
      Self::Generic3x3 { inv } => inv.lock().add(stack),
      Self::Crafting { grid, .. } => grid.lock().add(stack),
      _ => todo!(),
    }
  }
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
