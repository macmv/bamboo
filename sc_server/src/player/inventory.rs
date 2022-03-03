use crate::item::{Inventory, Stack};
use sc_common::net::sb::{Button, ClickWindow};
use std::mem;

/// An inventory, wrapped so that any time it is modified, a packet will be sent
/// to a client.
#[derive(Debug)]
pub struct WrappedInventory {
  inv: Inventory,
}

#[derive(Debug)]
pub struct PlayerInventory {
  main:           WrappedInventory,
  // An index into the hotbar (0..=8)
  selected_index: u8,
  // Open window and held item
  window:         Option<(WrappedInventory, Stack)>,
}

impl PlayerInventory {
  pub fn new() -> Self {
    // We always store an inventory with 46 slots, even if the client is on 1.8 (in
    // that version, there was no off-hand).
    PlayerInventory {
      main:           WrappedInventory::new(Inventory::new(46)),
      selected_index: 0,
      window:         None,
    }
  }

  pub fn open_window(&mut self, inv: Inventory) {
    assert!(self.window.is_none());
    self.window = Some((WrappedInventory::new(inv), Stack::empty()));
  }

  /// Returns the item in the player's main hand.
  pub fn main_hand(&self) -> &Stack { self.main().get(self.selected_index as u32 + 36) }

  /// Returns the currently selected hotbar index.
  pub fn selected_index(&self) -> u8 { self.selected_index }

  /// Sets the selected index. Should only be used when recieving a held item
  /// slot packet.
  pub(crate) fn set_selected(&mut self, index: u8) { self.selected_index = index; }

  pub fn main(&self) -> &WrappedInventory { &self.main }
  pub fn main_mut(&mut self) -> &mut WrappedInventory { &mut self.main }

  pub fn win(&self) -> Option<&WrappedInventory> { self.window.as_ref().map(|(win, _)| win) }
  pub fn win_mut(&mut self) -> Option<&mut WrappedInventory> {
    self.window.as_mut().map(|(win, _)| win)
  }

  /// Gets the item out of the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn get(&self, index: i32) -> &Stack {
    if index == -999 {
      &self.window.as_ref().unwrap().1
    } else if let Some((win, _)) = &self.window {
      if index > 0 {
        let i = index as u32;
        if i < win.size() {
          win.get(i)
        } else {
          self.main.get(i - win.size())
        }
      } else {
        self.main.get(index as u32)
      }
    } else {
      self.main.get(index as u32)
    }
  }
  /// Gets the item out of the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn get_mut(&mut self, index: i32) -> &mut Stack {
    if index == -999 {
      &mut self.window.as_mut().unwrap().1
    } else if let Some((win, _)) = &mut self.window {
      if index > 0 {
        let i = index as u32;
        if i < win.size() {
          win.get_mut(i)
        } else {
          self.main.get_mut(i - win.size())
        }
      } else {
        self.main.get_mut(index as u32)
      }
    } else {
      self.main.get_mut(index as u32)
    }
  }

  /// Replaces the item at `index` with the given item. The old item will be
  /// returned. This allows you to replace items without cloning them.
  pub fn replace(&mut self, index: i32, stack: Stack) -> Stack {
    mem::replace(self.get_mut(index), stack)
  }
  /// Sets the item in the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn set(&mut self, index: i32, stack: Stack) { *self.get_mut(index) = stack }

  /// Handles an inventory move operation.
  pub fn click_window(&mut self, slot: i32, click: ClickWindow) {
    info!("handling click at slot {slot} {click:?}");
    match click {
      ClickWindow::Click(button) => match button {
        Button::Left => self.swap(slot, -999),
        Button::Right => self.split(slot, -999),
        Button::Middle => todo!(),
      },
      ClickWindow::Number(num) => {
        self.swap(slot, (num + 36).into());
      }
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
}

impl WrappedInventory {
  pub fn new(inv: Inventory) -> Self { WrappedInventory { inv } }
  /// Gets the item at the given index.
  pub fn get(&self, index: u32) -> &Stack { self.inv.get(index as u32) }
  /// Gets the item at the given index.
  pub fn get_mut(&mut self, index: u32) -> &mut Stack { self.inv.get_mut(index as u32) }
  /// Sets the item in the inventory.
  ///
  /// TODO: Send a packet here.
  pub fn set(&mut self, index: u32, stack: Stack) { self.inv.set(index as u32, stack) }

  pub fn size(&self) -> u32 { self.inv.size() }
}
