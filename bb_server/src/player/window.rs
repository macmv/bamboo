use crate::{
  item::{SharedInventory, Stack},
  player::ConnSender,
};
use bb_common::util::UUID;

#[derive(bb_plugin_macros::Window, Debug, Clone)]
pub enum Window {
  #[name("minecraft:generic_9x1")]
  Generic9x1 { inv: SharedInventory<9> },
  #[name("minecraft:generic_9x2")]
  Generic9x2 { inv: SharedInventory<18> },
  #[name("minecraft:generic_9x3")]
  Generic9x3 { inv: SharedInventory<27> },
  #[name("minecraft:generic_9x4")]
  Generic9x4 { inv: SharedInventory<36> },
  #[name("minecraft:generic_9x5")]
  Generic9x5 { inv: SharedInventory<45> },
  #[name("minecraft:generic_9x6")]
  Generic9x6 { inv: SharedInventory<54> },
  #[name("minecraft:generic_3x3")]
  Generic3x3 { inv: SharedInventory<9> },
  #[name("minecraft:anvil")]
  Anvil {
    tool:    SharedInventory<1>,
    enchant: SharedInventory<1>,
    #[output]
    output:  SharedInventory<1>,
  },
  #[name("minecraft:beacon")]
  Beacon { inv: SharedInventory<1> },
  #[name("minecraft:blast_furnace")]
  BlastFurnace {
    input:  SharedInventory<1>,
    #[filter(fuel)]
    fuel:   SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:brewing_stand")]
  BrewingStand {
    bottles:    SharedInventory<3>,
    ingredient: SharedInventory<1>,
    fuel:       SharedInventory<1>,
  },
  #[name("minecraft:crafting")]
  Crafting {
    #[output]
    output: SharedInventory<1>,
    grid:   SharedInventory<9>,
    #[ignore]
    wm:     WorldManager,
  },
  #[name("minecraft:enchantment")]
  Enchantment { book: SharedInventory<1>, lapis: SharedInventory<1> },
  #[name("minecraft:furance")]
  Furnace {
    input:  SharedInventory<1>,
    #[filter(fuel)]
    fuel:   SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:grindstone")]
  Grindstone {
    inputs: SharedInventory<2>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:hopper")]
  Hopper { inv: SharedInventory<5> },
  #[name("minecraft:lectern")]
  Lectern { book: SharedInventory<1> },
  #[name("minecraft:loom")]
  Loom {
    banner:  SharedInventory<1>,
    dye:     SharedInventory<1>,
    pattern: SharedInventory<1>,
    #[output]
    output:  SharedInventory<1>,
  },
  #[name("minecraft:merchant")]
  Merchant { inv: SharedInventory<1> },
  #[name("minecraft:shulker_box")]
  ShulkerBox { inv: SharedInventory<27> },
  #[name("minecraft:smithing")]
  Smithing {
    input:   SharedInventory<1>,
    upgrade: SharedInventory<1>,
    #[output]
    output:  SharedInventory<1>,
  },
  #[name("minecraft:smoker")]
  Smoker {
    input:  SharedInventory<1>,
    #[filter(fuel)]
    fuel:   SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:cartography")]
  Cartography {
    map:    SharedInventory<1>,
    paper:  SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
  #[name("minecraft:stonecutter")]
  Stonecutter {
    input:  SharedInventory<1>,
    #[output]
    output: SharedInventory<1>,
  },
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
  pub fn set(&mut self, index: u32, stack: Stack) {
    self.access_mut(index, move |s| *s = stack);
    self.sync(index);
  }
  pub fn items(&self) -> ItemsIter<'_> { ItemsIter { win: self, index: 0 } }
}
