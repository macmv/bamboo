use crate::item::{Inventory, Stack};
use sc_common::net::sb::{Button, ClickWindow};
use std::mem;

#[derive(Debug)]
pub struct PlayerInventory {
  inv:            Inventory,
  // An index into the hotbar (0..=8)
  selected_index: u8,
  // The item on your cursor. Used when in an inventory.
  held:           Stack,
}

impl PlayerInventory {
  pub fn new() -> Self {
    PlayerInventory {
      inv:            Inventory::new(46),
      selected_index: 0,
      held:           Stack::empty(),
    }
  }

  /// Returns the item in the player's main hand.
  pub fn main_hand(&self) -> &Stack { self.inv.get(self.selected_index as u32 + 36) }

  /// Returns the currently selected hotbar index.
  pub fn selected_index(&self) -> u8 { self.selected_index }

  /// Sets the selected index. Should only be used when recieving a held item
  /// slot packet.
  pub(crate) fn set_selected(&mut self, index: u8) { self.selected_index = index; }

  /// Gets the item at the given index. 0 is part of the armor slots, not the
  /// start of the hotbar. To access the hotbar, add 36 to the index returned
  /// from main_hand.
  pub fn get(&self, index: i32) -> &Stack {
    if index == -999 {
      &self.held
    } else {
      self.inv.get(index as u32)
    }
  }
  /// Gets the item at the given index. 0 is part of the armor slots, not the
  /// start of the hotbar. To access the hotbar, add 36 to the index returned
  /// from main_hand.
  pub fn get_mut(&mut self, index: i32) -> &mut Stack {
    if index == -999 {
      &mut self.held
    } else {
      self.inv.get_mut(index as u32)
    }
  }

  /// Sets the item in the inventory.
  ///
  /// TODO: Send a packet here.
  pub fn set(&mut self, index: i32, stack: Stack) {
    if index == -999 {
      self.held = stack
    } else {
      self.inv.set(index as u32, stack)
    }
  }

  /// Handles an inventory move operation.
  pub fn click_window(&mut self, slot: i32, click: ClickWindow) {
    match click {
      ClickWindow::Click(button) => match button {
        Button::Left => self.swap(slot, -999),
        Button::Right => self.split(slot, -999),
        Button::Middle => todo!(),
      },
      _ => todo!(),
    }
  }

  /// Takes half of the items in the slot `a` and moves them to `b`. If `b` is
  /// not empty, this is a noop.
  pub fn split(&mut self, a: i32, b: i32) {
    if self.get(b).is_empty() {
      let stack = self.get_mut(a);
      let total = stack.amount();
      stack.set_amount(total / 2);
      let remaining = total - stack.amount();
      self.set(b, self.get(a).clone().with_amount(remaining));
    }
  }

  /// Swaps the items at index `a` and `b`.
  pub fn swap(&mut self, a: i32, b: i32) {
    let a_it = self.replace(a, Stack::empty());
    let b_it = self.replace(b, a_it);
    self.set(a, b_it);
  }

  /// Replaces the item at `index` with the given item. The old item will be
  /// returned. This allows you to replace items without cloning them.
  pub fn replace(&mut self, index: i32, stack: Stack) -> Stack {
    if index == -999 {
      mem::replace(&mut self.held, stack)
    } else {
      self.inv.replace(index as u32, stack)
    }
  }
}
