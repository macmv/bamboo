use super::Inventory;

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
  Merchant {},
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
