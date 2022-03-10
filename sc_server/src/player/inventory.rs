use crate::{
  entity,
  item::{Inventory, Stack},
  net::ConnSender,
  player::Player,
};
use sc_common::{
  nbt::{Tag, NBT},
  net::{
    cb,
    sb::{Button, ClickWindow},
  },
};
use std::{
  mem,
  sync::{Arc, Weak},
};

/// An inventory, wrapped so that any time it is modified, a packet will be sent
/// to a client.
#[derive(Debug)]
pub struct WrappedInventory {
  inv:    Inventory,
  conn:   ConnSender,
  offset: u32,
  skip:   u32,
}

#[derive(Debug)]
pub struct PlayerInventory {
  // The main inventory. Slots 0 through 8 are your armor and crafting bench slots. Slots 9-44 are
  // the main inventory. Slot 45 is the off hand.
  //
  // Note that slots 9-44 are used in chests as well, so we have some extra nonsense to handle
  // that.
  main:           WrappedInventory,
  // An index into the hotbar (0..=8)
  selected_index: u8,
  // Open window and held item
  window:         Option<(WrappedInventory, Stack)>,

  /// Used when draging the mouse over items. This is sent as a collection of
  /// packets all at once, after the drag is finished. Because it's sent as
  /// multiple packets, we need to hold onto this state.
  drag_slots: Vec<i32>,

  player: Weak<Player>,
}

impl PlayerInventory {
  pub fn new(weak: Weak<Player>, conn: ConnSender) -> Self {
    // We always store an inventory with 46 slots, even if the client is on 1.8 (in
    // that version, there was no off-hand).
    PlayerInventory {
      main:           WrappedInventory::new(Inventory::new(46), conn),
      selected_index: 0,
      window:         None,
      drag_slots:     vec![],
      player:         weak,
    }
  }

  pub fn open_window(&mut self, inv: Inventory) {
    assert!(self.window.is_none());
    self.main.set_offset_skip(inv.size(), 9);
    self.window = Some((WrappedInventory::new(inv, self.main.conn.clone()), Stack::empty()));
  }
  pub fn close_window(&mut self) {
    self.window.take();
    self.main.set_offset_skip(0, 0);
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
      if index >= 0 {
        let i = index as u32;
        if i < win.size() {
          win.get(i)
        } else {
          // Note the +9. This is because slots 0-8 are the armor slots of the player's
          // inventory.
          self.main.get(i - win.size() + 9)
        }
      } else {
        self.main.get(index as u32)
      }
    } else {
      self.main.get(index as u32)
    }
  }
  // This is private as modifying the stack doesn't send an update to the client.
  fn get_mut(&mut self, index: i32) -> &mut Stack {
    if index == -999 {
      &mut self.window.as_mut().unwrap().1
    } else if let Some((win, _)) = &mut self.window {
      if index >= 0 {
        let i = index as u32;
        if i < win.size() {
          win.get_mut(i)
        } else {
          // Note the +9. This is because slots 0-8 are the armor slots of the player's
          // inventory.
          self.main.get_mut(i - win.size() + 9)
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
    let res = mem::replace(self.get_mut(index), stack);
    self.sync(index);
    res
  }
  /// Sets the item in the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn set(&mut self, index: i32, stack: Stack) {
    *self.get_mut(index) = stack;
    self.sync(index);
  }

  /// Sends an inventory update to the client. This is more efficient than
  /// calling [`sync`](Self::sync) for all the slots in the inventory, but is
  /// less efficient than syncing a single slot. Only use this when needed, as
  /// it will send the data for every item to the client.
  pub fn sync_all(&self) {
    let mut items = vec![];
    let mut held = Stack::empty().to_item();
    if let Some((inv, win_held)) = &self.window {
      held = win_held.to_item();
      for it in inv.inv.items() {
        items.push(it.to_item());
      }
    }
    // Skip the armor/crafting bench slots
    for it in self.main.inv.items().iter().skip(9) {
      items.push(it.to_item());
    }
    self.main.conn.send(cb::Packet::WindowItems { wid: 1, items, held });
  }
  /// Sends an item update for the given slot. This shouldn't every be needed,
  /// as functions like [`set`](Self::set) and [`replace`](Self::replace) will
  /// call this for you.
  pub fn sync(&self, index: i32) {
    if index == -999 {
      self.main.conn.send(cb::Packet::WindowItem {
        wid:  u8::MAX,
        slot: -1,
        item: self.window.as_ref().unwrap().1.to_item(),
      });
    } else if let Some((win, _)) = &self.window {
      if index >= 0 {
        let i = index as u32;
        if i < win.size() {
          win.sync(i)
        } else {
          self.main.sync(i - win.size() + 9)
        }
      } else {
        self.main.sync(index as u32)
      }
    } else {
      self.main.sync(index as u32)
    }
  }

  /// Handles an inventory move operation.
  pub fn click_window(&mut self, slot: i32, click: ClickWindow, allow: bool) {
    info!("handling click at slot {slot} {click:?}");

    macro_rules! allow {
      (self.$name:ident($($arg:expr),*)) => {
        if allow {
          self.$name($($arg),*);
        } else {
          $(
            self.sync($arg);
          )*
        }
      };
    }

    let inv_size = self.win().unwrap().size();
    let in_main = slot >= 0 && (slot as u32) > inv_size;
    info!("in main: {in_main}");
    match click {
      ClickWindow::Click(button) => {
        if slot == -999 {
          match button {
            Button::Left => allow!(self.drop_all(slot)),
            Button::Right => allow!(self.drop_one(slot)),
            Button::Middle => {}
          }
        } else {
          match button {
            Button::Left => allow!(self.swap(slot, -999)),
            Button::Right => allow!(self.split(slot, -999)),
            Button::Middle => {}
          }
        }
      }
      ClickWindow::ShiftClick(_) => {
        let stack = self.get(slot).clone();
        let prev_amount = stack.amount();
        // We are shift clicking in `in_main`, so we add it to the other inventory.
        if allow {
          let new_amount =
            if in_main { self.win_mut().unwrap().add(&stack) } else { self.main.add(&stack) };
          if new_amount != prev_amount {
            self.set(slot, stack.with_amount(new_amount));
          }
          self.sync_all();
        } else {
          let new_amount = if in_main {
            self.win_mut().unwrap().add_sync(&stack)
          } else {
            self.main.add_sync(&stack)
          };
          if new_amount != prev_amount {
            self.sync(slot);
          }
        }
      }
      ClickWindow::Number(num) => allow!(self.swap(slot, num as i32 + 27 + inv_size as i32)),
      ClickWindow::Drop => allow!(self.drop_one(slot)),
      ClickWindow::DropAll => allow!(self.drop_all(slot)),
      ClickWindow::DoubleClick => allow!(self.double_click(slot)),
      ClickWindow::DragStart(_) => self.drag_start(),
      ClickWindow::DragAdd(_) => self.drag_add(slot),
      ClickWindow::DragEnd(_) => self.drag_end(),
    }

    self.sync_all();
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

  /// Removes a single item from the given slot.
  pub fn drop_one(&mut self, slot: i32) {
    let it = self.get(slot);
    if it.is_empty() || it.amount() == 0 {
      return;
    }
    let _old = if it.amount() == 1 {
      self.replace(slot, Stack::empty())
    } else {
      let it = self.get_mut(slot);
      it.set_amount(it.amount() - 1);
      self.sync(slot);
      self.get(slot).clone()
    };
    if let Some(p) = self.player.upgrade() {
      let mut tag = NBT::new(
        "",
        Tag::compound(&[(
          "Item",
          Tag::compound(&[("id", Tag::String("minecraft:diamond".into()))]),
        )]),
      );
      p.world().summon_nbt(entity::Type::Item, p.pos(), tag);
    }
  }

  /// Removes the entire stack at the given slot.
  pub fn drop_all(&mut self, slot: i32) {
    let _old = self.replace(slot, Stack::empty());
    if let Some(p) = self.player.upgrade() {
      p.world().summon(entity::Type::Item, p.pos());
    }
  }

  /// Grabs up to a full stack of whatever item is at the given slot, and moves
  /// it to the cursor slot.
  pub fn double_click(&mut self, _slot: i32) {
    let mut held = self.replace(-999, Stack::empty());
    for i in 0..36 + self.win().unwrap().size() {
      let i = i as i32;
      let stack = self.get(i);
      if stack.item() != held.item() {
        continue;
      }
      if stack.amount() + held.amount() > 64 {
        let mut stack = stack.clone();
        stack.set_amount(stack.amount() - (64 - held.amount()));
        self.set(i, stack);
        held.set_amount(64);
      } else {
        let amount = stack.amount();
        self.set(i, Stack::empty());
        held.set_amount(held.amount() + amount);
      }
      if held.amount() == 64 {
        break;
      }
    }
    self.set(-999, held);
  }

  pub fn drag_start(&mut self) { self.drag_slots.clear(); }
  pub fn drag_add(&mut self, slot: i32) { self.drag_slots.push(slot); }
  pub fn drag_end(&mut self) {
    let stack = self.get(-999).clone();
    let items_per_slot = stack.amount() / self.drag_slots.len() as u8;
    let items_remaining = stack.amount() % self.drag_slots.len() as u8;
    for slot in self.drag_slots.clone() {
      self.set(slot, stack.clone().with_amount(items_per_slot));
    }
    self.drag_slots.clear();
    if items_remaining == 0 {
      self.set(-999, Stack::empty());
    } else {
      let stack = self.get_mut(-999);
      stack.set_amount(items_remaining);
      self.sync(-999);
    }
  }
}

impl WrappedInventory {
  pub fn new(inv: Inventory, conn: ConnSender) -> Self {
    WrappedInventory { inv, conn, offset: 0, skip: 0 }
  }
  /// Gets the item at the given index.
  pub fn get(&self, index: u32) -> &Stack { self.inv.get(index as u32) }
  /// This is private as updating the item doesn't send an update to the client.
  fn get_mut(&mut self, index: u32) -> &mut Stack { self.inv.get_mut(index as u32) }
  /// Sets the item in the inventory.
  pub fn set(&mut self, index: u32, stack: Stack) {
    *self.get_mut(index) = stack;
    self.sync(index);
  }
  /// Replaces an item in the inventory.
  pub fn replace(&mut self, index: u32, stack: Stack) -> Stack {
    let res = mem::replace(self.get_mut(index), stack);
    self.sync(index);
    res
  }
  /// Syncs the item at the given slot with the client.
  pub fn sync(&self, index: u32) {
    self.conn.send(cb::Packet::WindowItem {
      wid:  1,
      slot: (index + self.offset - self.skip) as i32,
      item: self.inv.get(index).to_item(),
    });
  }

  /// Tries to add the given stack to this inventory. This will return the
  /// number of remaining items in the stack. If the inventory has enough space,
  /// this will return 0.
  pub fn add(&mut self, stack: &Stack) -> u8 {
    // Local `sync` impl, which doesn't use the internal inventory. This is needed
    // because we iterate through the inventory with `items_mut`, so the inventory
    // is mutably borrowed for the entire loop.
    let sync = |index: u32, item: &Stack| {
      self.conn.send(cb::Packet::WindowItem {
        wid:  1,
        slot: (index + self.offset - self.skip) as i32,
        item: item.to_item(),
      });
    };
    let mut remaining = stack.amount();
    for (i, it) in self.inv.items_mut().iter_mut().skip(self.skip as usize).enumerate() {
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
    for (i, it) in self.inv.items().iter().skip(self.skip as usize).enumerate() {
      let i = i as u32;
      if it.is_empty() {
        self.sync(i);
        remaining = 0;
      } else if it.item() == stack.item() {
        let amount_possible = 64 - it.amount();
        if amount_possible > remaining {
          remaining = 0;
        } else {
          remaining -= amount_possible;
        }
        self.sync(i);
      }
      if remaining == 0 {
        break;
      }
    }
    remaining
  }

  /// Sets the offset for sending packets. This is used when an inventory is
  /// opened/closed. It causes [`set`](Self::set) to send slot ids with this
  /// offset.
  ///
  /// The `skip` is used when adding items; it is used in the main inventory
  /// to skip the armor slots.
  ///
  /// Note that the actual id sent will be `offset - skip`. So for the main
  /// inventory, when shown a chest, the offset should be set to 27, and skip
  /// should be sent to 9. This means the actual ids sent will be offset by 18.
  fn set_offset_skip(&mut self, offset: u32, skip: u32) {
    self.offset = offset;
    self.skip = skip;
  }

  pub fn size(&self) -> u32 { self.inv.size() }
}
