use super::Stack;
use crate::net::ConnSender;
use bb_common::net::cb;
use std::mem;

/// An inventory. This is a very abstract concept in Bamboo. Unlike vanilla,
/// which uses a bunch of hardocoded offsets into one inventory, we use multiple
/// inventories in 1 window for bamboo. Each inventory has an offset, so that it
/// can send packets with the correct ids.
///
/// The inventories are seperated to make shift-click logic very simple to
/// write.
#[derive(Debug, Clone)]
pub struct Inventory<const N: usize> {
  items: [Stack; N],
}

impl<const N: usize> Default for Inventory<N> {
  fn default() -> Self { Self::new() }
}

impl<const N: usize> Inventory<N> {
  pub fn new() -> Self { Inventory { items: [Stack::EMPTY; N] } }

  /// Sets an item in the inventory.
  #[track_caller]
  pub fn set(&mut self, index: u32, stack: Stack) { self.items[index as usize] = stack; }
  /// Returns a reference to the item stack at the given index.
  pub fn get(&self, index: u32) -> Option<&Stack> { self.items.get(index as usize) }
  /// Returns a mutable reference to the item stack at the given index.
  pub fn get_mut(&mut self, index: u32) -> Option<&mut Stack> { self.items.get_mut(index as usize) }

  /// Returns the inventory size.
  pub const fn size(&self) -> u32 { self.items.len() as u32 }
  /// Returns the items in the inventory.
  pub fn items(&self) -> &[Stack; N] { &self.items }
  /// Returns the items in the inventory.
  pub fn items_mut(&mut self) -> &mut [Stack; N] { &mut self.items }

  /// Replaces the item at `index` with the given stack.
  pub fn replace(&mut self, index: u32, stack: Stack) -> Stack {
    mem::replace(&mut self.items[index as usize], stack)
  }

  /// Tries to add the given stack to this inventory. This will return the
  /// number of remaining items in the stack. If the inventory has enough space,
  /// this will return 0.
  pub fn add(&mut self, stack: &Stack) -> u8 {
    let mut remaining = stack.amount();
    for it in self.items_mut().iter_mut() {
      if it.is_empty() {
        *it = stack.clone().with_amount(remaining);
        remaining = 0;
      } else if it.item() == stack.item() {
        let amount_possible = 64 - it.amount();
        if amount_possible > remaining {
          *it = stack.clone().with_amount(it.amount() + remaining);
          remaining = 0;
        } else {
          *it = stack.clone().with_amount(64);
          remaining -= amount_possible;
        }
      }
      if remaining == 0 {
        break;
      }
    }
    remaining
  }
}

/// An inventory, wrapped so that any time it is modified, a packet will be sent
/// to a client.
#[derive(Debug)]
pub struct SingleInventory<const N: usize> {
  pub(crate) inv:    Inventory<N>,
  pub(crate) conn:   ConnSender,
  pub(crate) wid:    u8,
  pub(crate) offset: u32,
}

#[derive(Debug)]
pub struct WrappedInventory<const N: usize> {
  pub(crate) inv:     Inventory<N>,
  // Everyone who has this inventory open.
  pub(crate) viewers: Vec<ConnSender>,
  pub(crate) wid:     u8,
  pub(crate) offset:  u32,
}

impl<const N: usize> SingleInventory<N> {
  pub fn new(conn: ConnSender, wid: u8, offset: u32) -> Self {
    SingleInventory { inv: Inventory::new(), conn, wid, offset }
  }
  /// Gets the item at the given index.
  pub fn get_raw(&self, index: u32) -> Option<&Stack> { self.inv.get(index) }
  /// Given an index with an offset, this will remove the offset, and lookup the
  /// item at that index.
  pub fn get(&self, index: u32) -> Option<&Stack> {
    if index < self.offset {
      None
    } else {
      self.get_raw(index - self.offset)
    }
  }
  /// This is private as updating the item doesn't send an update to the client.
  pub(crate) fn get_raw_mut(&mut self, index: u32) -> Option<&mut Stack> {
    self.inv.get_mut(index as u32)
  }
  /// Given an index with an offset, this will remove the offset, and lookup the
  /// item at that index.
  pub(crate) fn get_mut(&mut self, index: u32) -> Option<&mut Stack> {
    if index < self.offset {
      None
    } else {
      self.get_raw_mut(index - self.offset)
    }
  }
  /// Sets the item in the inventory.
  #[track_caller]
  pub fn set_raw(&mut self, index: u32, stack: Stack) {
    if let Some(it) = self.get_raw_mut(index) {
      *it = stack;
      self.sync_raw(index);
    } else {
      panic!("index too large {} > {}", index, self.size());
    }
  }
  /// Sets the item in the inventory, offsetting the index by self.offset.
  #[track_caller]
  pub fn set(&mut self, index: u32, stack: Stack) {
    if index < self.offset {
      panic!("index too small {} < {}", index, self.offset);
    } else {
      self.set_raw(index - self.offset, stack);
    }
  }
  /// Replaces an item in the inventory.
  #[track_caller]
  pub fn replace_raw(&mut self, index: u32, stack: Stack) -> Stack {
    let res = mem::replace(self.get_raw_mut(index).unwrap(), stack);
    self.sync_raw(index);
    res
  }
  /// Syncs the item at the given slot with the client.
  #[track_caller]
  pub fn sync_raw(&self, index: u32) {
    self.conn.send(cb::Packet::WindowItem {
      wid:  self.wid,
      slot: (index + self.offset) as i32,
      item: self.get_raw(index).unwrap().to_item(),
    });
  }
  /// Syncs the item at the index. The offset will first be subtracted from the
  /// index, to get the actual inventory offset.
  #[track_caller]
  pub(crate) fn sync(&self, index: u32) {
    self.conn.send(cb::Packet::WindowItem {
      wid:  self.wid,
      slot: index as i32,
      item: self.get(index).unwrap().to_item(),
    });
  }

  /// Returns true if the given slot is within `self.offset..self.offset +
  /// self.size`.
  pub fn has_slot(&self, index: u32) -> bool { self.get(index).is_some() }

  /// Tries to add the given stack to this inventory. This will return the
  /// number of remaining items in the stack. If the inventory has enough space,
  /// this will return 0.
  pub fn add(&mut self, stack: &Stack) -> u8 {
    // Local `sync` impl, which doesn't use the internal inventory. This is needed
    // because we iterate through the inventory with `items_mut`, so the inventory
    // is mutably borrowed for the entire loop.
    let sync = |index: u32, item: &Stack| {
      self.conn.send(cb::Packet::WindowItem {
        wid:  self.wid,
        slot: (index + self.offset) as i32,
        item: item.to_item(),
      });
    };
    let mut remaining = stack.amount();
    for (i, it) in self.inv.items_mut().iter_mut().enumerate() {
      let i = i as u32;
      if it.is_empty() {
        *it = stack.clone().with_amount(remaining);
        sync(i, it);
        remaining = 0;
      } else if it.item() == stack.item() {
        let amount_possible = 64 - it.amount();
        if amount_possible > remaining {
          *it = stack.clone().with_amount(it.amount() + remaining);
          remaining = 0;
        } else {
          *it = stack.clone().with_amount(64);
          remaining -= amount_possible;
        }
        sync(i, it);
      }
      if remaining == 0 {
        break;
      }
    }
    remaining
  }
  /// This does the same thing as `add`, but doesn't move any items around. This
  /// is used when the client shift clicks an item, and the server declines the
  /// transaction.
  pub fn add_sync(&self, stack: &Stack) -> u8 {
    let mut remaining = stack.amount();
    for (i, it) in self.inv.items().iter().enumerate() {
      let i = i as u32;
      if it.is_empty() {
        self.sync_raw(i);
        remaining = 0;
      } else if it.item() == stack.item() {
        let amount_possible = 64 - it.amount();
        if amount_possible > remaining {
          remaining = 0;
        } else {
          remaining -= amount_possible;
        }
        self.sync_raw(i);
      }
      if remaining == 0 {
        break;
      }
    }
    remaining
  }

  /// Returns the size of this inventory.
  pub const fn size(&self) -> u32 { self.inv.size() }
}

impl<const N: usize> WrappedInventory<N> {
  pub fn new(wid: u8, offset: u32) -> Self {
    WrappedInventory { inv: Inventory::new(), viewers: vec![], wid, offset }
  }

  pub fn open(&mut self, conn: ConnSender) { self.viewers.push(conn); }
  /// Gets the item at the given index.
  pub fn get_raw(&self, index: u32) -> Option<&Stack> { self.inv.get(index) }
  /// Given an index with an offset, this will remove the offset, and lookup the
  /// item at that index.
  pub fn get(&self, index: u32) -> Option<&Stack> {
    if index < self.offset {
      None
    } else {
      self.get_raw(index - self.offset)
    }
  }
  /// This is private as updating the item doesn't send an update to the client.
  pub(crate) fn get_raw_mut(&mut self, index: u32) -> Option<&mut Stack> {
    self.inv.get_mut(index as u32)
  }
  /// Given an index with an offset, this will remove the offset, and lookup the
  /// item at that index.
  pub(crate) fn get_mut(&mut self, index: u32) -> Option<&mut Stack> {
    if index < self.offset {
      None
    } else {
      self.get_raw_mut(index - self.offset)
    }
  }
  /// Sets the item in the inventory.
  #[track_caller]
  pub fn set_raw(&mut self, index: u32, stack: Stack) {
    if let Some(it) = self.get_raw_mut(index) {
      *it = stack;
      self.sync_raw(index);
    } else {
      panic!("index too large {} > {}", index, self.size());
    }
  }
  /// Sets the item in the inventory, offsetting the index by self.offset.
  #[track_caller]
  pub fn set(&mut self, index: u32, stack: Stack) {
    if index < self.offset {
      panic!("index too small {} < {}", index, self.offset);
    } else {
      self.set_raw(index - self.offset, stack);
    }
  }
  /// Replaces an item in the inventory.
  #[track_caller]
  pub fn replace_raw(&mut self, index: u32, stack: Stack) -> Stack {
    let res = mem::replace(self.get_raw_mut(index).unwrap(), stack);
    self.sync_raw(index);
    res
  }
  /// Syncs the item at the given slot with the client.
  #[track_caller]
  pub fn sync_raw(&self, index: u32) {
    for conn in &self.viewers {
      conn.send(cb::Packet::WindowItem {
        wid:  self.wid,
        slot: (index + self.offset) as i32,
        item: self.get_raw(index).unwrap().to_item(),
      });
    }
  }
  /// Syncs the item at the index. The offset will first be subtracted from the
  /// index, to get the actual inventory offset.
  #[track_caller]
  pub(crate) fn sync(&self, index: u32) {
    for conn in &self.viewers {
      conn.send(cb::Packet::WindowItem {
        wid:  self.wid,
        slot: index as i32,
        item: self.get(index).unwrap().to_item(),
      });
    }
  }

  /// Returns true if the given slot is within `self.offset..self.offset +
  /// self.size`.
  pub fn has_slot(&self, index: u32) -> bool { self.get(index).is_some() }

  /// Tries to add the given stack to this inventory. This will return the
  /// number of remaining items in the stack. If the inventory has enough space,
  /// this will return 0.
  pub fn add(&mut self, stack: &Stack) -> u8 {
    // Local `sync` impl, which doesn't use the internal inventory. This is needed
    // because we iterate through the inventory with `items_mut`, so the inventory
    // is mutably borrowed for the entire loop.
    let sync = |index: u32, item: &Stack| {
      for conn in &self.viewers {
        conn.send(cb::Packet::WindowItem {
          wid:  self.wid,
          slot: (index + self.offset) as i32,
          item: item.to_item(),
        });
      }
    };
    let mut remaining = stack.amount();
    for (i, it) in self.inv.items_mut().iter_mut().enumerate() {
      let i = i as u32;
      if it.is_empty() {
        *it = stack.clone().with_amount(remaining);
        sync(i, it);
        remaining = 0;
      } else if it.item() == stack.item() {
        let amount_possible = 64 - it.amount();
        if amount_possible > remaining {
          *it = stack.clone().with_amount(it.amount() + remaining);
          remaining = 0;
        } else {
          *it = stack.clone().with_amount(64);
          remaining -= amount_possible;
        }
        sync(i, it);
      }
      if remaining == 0 {
        break;
      }
    }
    remaining
  }
  /// This does the same thing as `add`, but doesn't move any items around. This
  /// is used when the client shift clicks an item, and the server declines the
  /// transaction.
  pub fn add_sync(&self, stack: &Stack) -> u8 {
    let mut remaining = stack.amount();
    for (i, it) in self.inv.items().iter().enumerate() {
      let i = i as u32;
      if it.is_empty() {
        self.sync_raw(i);
        remaining = 0;
      } else if it.item() == stack.item() {
        let amount_possible = 64 - it.amount();
        if amount_possible > remaining {
          remaining = 0;
        } else {
          remaining -= amount_possible;
        }
        self.sync_raw(i);
      }
      if remaining == 0 {
        break;
      }
    }
    remaining
  }

  /// Returns the size of this inventory.
  pub const fn size(&self) -> u32 { self.inv.size() }
}
