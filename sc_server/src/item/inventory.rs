use super::Stack;
use std::mem;

#[derive(Debug, Clone)]
pub struct Inventory {
  items: Vec<Stack>,
}

impl Inventory {
  pub fn new(size: u32) -> Self { Inventory { items: vec![Stack::empty(); size as usize] } }

  /// Sets an item in the inventory.
  pub fn set(&mut self, index: u32, stack: Stack) { self.items[index as usize] = stack; }
  /// Returns a reference to the item stack at the given index.
  pub fn get(&self, index: u32) -> &Stack { &self.items[index as usize] }
  /// Returns a mutable reference to the item stack at the given index.
  pub fn get_mut(&mut self, index: u32) -> &mut Stack { &mut self.items[index as usize] }

  /// Returns the inventory size.
  pub fn size(&self) -> u32 { self.items.len() as u32 }
  /// Returns the items in the inventory.
  pub fn items(&self) -> &Vec<Stack> { &self.items }

  /// Replaces the item at `index` with the given stack.
  pub fn replace(&mut self, index: u32, stack: Stack) -> Stack {
    mem::replace(&mut self.items[index as usize], stack)
  }
}
