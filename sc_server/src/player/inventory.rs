use crate::{
  item::{Inventory, Stack},
  net::ConnSender,
};
use sc_common::net::{
  cb,
  sb::{Button, ClickWindow},
};
use std::mem;

/// An inventory, wrapped so that any time it is modified, a packet will be sent
/// to a client.
#[derive(Debug)]
pub struct WrappedInventory {
  inv:    Inventory,
  conn:   ConnSender,
  offset: u32,
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
  pub fn new(conn: ConnSender) -> Self {
    // We always store an inventory with 46 slots, even if the client is on 1.8 (in
    // that version, there was no off-hand).
    PlayerInventory {
      main:           WrappedInventory::new(Inventory::new(46), conn),
      selected_index: 0,
      window:         None,
    }
  }

  pub fn open_window(&mut self, inv: Inventory) {
    assert!(self.window.is_none());
    self.main.set_offset(inv.size());
    self.window = Some((WrappedInventory::new(inv, self.main.conn.clone()), Stack::empty()));
  }
  pub fn close_window(&mut self) {
    let (_, _) = self.window.take().unwrap();
    self.main.set_offset(0);
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
  /// Replaces the item at `index` with the given item. The old item will be
  /// returned. This allows you to replace items without cloning them.
  pub fn replace(&mut self, index: i32, stack: Stack) -> Stack {
    if index == -999 {
      self.main.conn.send(cb::Packet::WindowItem {
        wid:  u8::MAX,
        slot: -1,
        item: stack.to_item(),
      });
      mem::replace(&mut self.window.as_mut().unwrap().1, stack)
    } else if let Some((win, _)) = &mut self.window {
      if index > 0 {
        let i = index as u32;
        if i < win.size() {
          win.replace(i, stack)
        } else {
          self.main.replace(i - win.size(), stack)
        }
      } else {
        self.main.replace(index as u32, stack)
      }
    } else {
      self.main.replace(index as u32, stack)
    }
  }
  /// Sets the item in the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn set(&mut self, index: i32, stack: Stack) {
    if index == -999 {
      self.main.conn.send(cb::Packet::WindowItem {
        wid:  u8::MAX,
        slot: -1,
        item: stack.to_item(),
      });
      self.window.as_mut().unwrap().1 = stack;
    } else if let Some((win, _)) = &mut self.window {
      if index > 0 {
        let i = index as u32;
        if i < win.size() {
          win.set(i, stack)
        } else {
          self.main.set(i - win.size(), stack)
        }
      } else {
        self.main.set(index as u32, stack)
      }
    } else {
      self.main.set(index as u32, stack)
    }
  }

  pub fn sync(&self, index: i32) {
    if index == -999 {
      self.main.conn.send(cb::Packet::WindowItem {
        wid:  u8::MAX,
        slot: -1,
        item: self.window.as_ref().unwrap().1.to_item(),
      });
    } else if let Some((win, _)) = &self.window {
      if index > 0 {
        let i = index as u32;
        if i < win.size() {
          win.sync(i)
        } else {
          self.main.sync(i - win.size())
        }
      } else {
        self.main.sync(index as u32)
      }
    } else {
      self.main.sync(index as u32)
    }
  }

  /// Handles an inventory move operation.
  pub fn click_window(&mut self, slot: i32, click: ClickWindow) {
    info!("handling click at slot {slot} {click:?}");
    let inv_size = self.win().unwrap().size();
    match click {
      ClickWindow::Click(button) => match button {
        Button::Left => self.swap(slot, -999),
        Button::Right => self.split(slot, -999),
        Button::Middle => todo!(),
      },
      ClickWindow::Number(num) => {
        // self.swap(slot, num as i32 + 27 + inv_size as i32);
        self.sync(slot);
        self.sync(num as i32 + 27 + inv_size as i32);
      }
      _ => todo!(),
    }
  }

  /// Takes half of the items in the slot `a` and moves them to `b`. If `b` is
  /// not empty, this is a noop.
  pub fn split(&mut self, a: i32, b: i32) {
    if self.get(b).is_empty() {
      let mut stack = self.get(a).clone();
      let total = stack.amount();
      stack.set_amount(total / 2);
      let remaining = total - stack.amount();
      self.set(a, stack);
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
  pub fn new(inv: Inventory, conn: ConnSender) -> Self { WrappedInventory { inv, conn, offset: 0 } }
  /// Gets the item at the given index.
  pub fn get(&self, index: u32) -> &Stack { self.inv.get(index as u32) }
  /// Sets the item in the inventory.
  pub fn set(&mut self, index: u32, stack: Stack) {
    self.conn.send(cb::Packet::WindowItem {
      wid:  1,
      slot: (index + self.offset) as i32,
      item: stack.to_item(),
    });
    self.inv.set(index as u32, stack);
  }
  /// Replaces an item in the inventory.
  pub fn replace(&mut self, index: u32, stack: Stack) -> Stack {
    self.conn.send(cb::Packet::WindowItem {
      wid:  1,
      slot: (index + self.offset) as i32,
      item: stack.to_item(),
    });
    self.inv.replace(index as u32, stack)
  }
  /// Syncs the item at the given slot with the client.
  pub fn sync(&self, index: u32) {
    self.conn.send(cb::Packet::WindowItem {
      wid:  1,
      slot: (index + self.offset) as i32,
      item: self.inv.get(index).to_item(),
    });
  }

  /// Sets the offset for sending packets. This is used when an inventory is
  /// opened/closed. It causes [`set`](Self::set) to send slot ids with this
  /// offset.
  fn set_offset(&mut self, offset: u32) { self.offset = offset; }

  pub fn size(&self) -> u32 { self.inv.size() }
}
