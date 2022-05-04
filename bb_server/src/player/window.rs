use crate::{
  item::{SharedInventory, Stack},
  player::ConnSender,
  world::WorldManager,
};
use bb_common::util::UUID;
use std::sync::Arc;

trait WindowData {
  fn get(&self, index: u32) -> Option<&Stack> { None }
  fn sync(&self, index: u32) {}
  fn access<R>(&self, index: u32, f: impl FnOnce(&Stack) -> R) -> Option<R> { None }
  fn access_mut<R>(&mut self, index: u32, f: impl FnOnce(&mut Stack) -> R) -> Option<R> { None }
  fn size(&self) -> u32 { 1 }
  fn add(&mut self, stack: Stack) -> u8 { 0 }
  fn open(&self, id: UUID, conn: &ConnSender) {}
  fn close(&self, id: UUID) {}
}

trait WindowHandler<T> {
  fn on_update(&self, inv: &T) { let _ = inv; }
}

#[derive(bb_plugin_macros::Window, Debug, Clone)]
#[handler(NoneHandler)]
pub struct GenericWindow<const N: usize> {
  pub inv: SharedInventory<N>,
}

#[derive(bb_plugin_macros::Window, Debug, Clone)]
#[handler(NoneHandler)]
pub struct SmeltingWindow {
  pub input:  SharedInventory<1>,
  // #[filter(fuel)]
  pub fuel:   SharedInventory<1>,
  #[output]
  pub output: SharedInventory<1>,
}

#[derive(bb_plugin_macros::Window, Debug, Clone)]
#[handler(CraftingWindowHandler)]
pub struct CraftingWindow {
  #[output]
  pub output: SharedInventory<1>,
  pub grid:   SharedInventory<9>,
  #[not_inv]
  pub wm:     Arc<WorldManager>,
}

struct NoneHandler;
impl<const N: usize> WindowHandler<GenericWindow<N>> for NoneHandler {}
impl WindowHandler<SmeltingWindow> for NoneHandler {}

struct CraftingWindowHandler;
impl WindowHandler<CraftingWindow> for CraftingWindowHandler {
  fn on_update(&self, win: &CraftingWindow) {
    info!("crafting window update: {:?}", win);
  }
}

#[derive(bb_plugin_macros::WindowEnum, Debug, Clone)]
pub enum Window {
  #[name("minecraft:generic_9x1")]
  Generic9x1(GenericWindow<9>),
  #[name("minecraft:generic_9x2")]
  Generic9x2(GenericWindow<18>),
  #[name("minecraft:generic_9x3")]
  Generic9x3(GenericWindow<27>),
  #[name("minecraft:generic_9x4")]
  Generic9x4(GenericWindow<36>),
  #[name("minecraft:generic_9x5")]
  Generic9x5(GenericWindow<45>),
  #[name("minecraft:generic_9x6")]
  Generic9x6(GenericWindow<54>),
  #[name("minecraft:generic_3x3")]
  Generic3x3(GenericWindow<9>),
  #[name("minecraft:crafting")]
  Crafting(CraftingWindow),
  /*
  #[name("minecraft:anvil")]
  Anvil(Anvil),
  #[name("minecraft:beacon")]
  Beacon(inv: SharedInventory<1>),
  #[name("minecraft:blast_furnace")]
  BlastFurnace(SmeltingWindow),
  #[name("minecraft:brewing_stand")]
  BrewingStand {
    bottles:    SharedInventory<3>,
    ingredient: SharedInventory<1>,
    fuel:       SharedInventory<1>,
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
  */
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
