use crate::{
  entity, item,
  item::{Inventory, Stack, WrappedInventory},
  net::ConnSender,
  player::Player,
};
use bb_common::{
  math::FPos,
  metadata::Metadata,
  net::{
    cb,
    sb::{Button, ClickWindow},
  },
  util::Hand,
};
use std::{mem, sync::Weak};

#[derive(Debug)]
pub struct PlayerInventory {
  head:           WrappedInventory<1>,
  chest:          WrappedInventory<1>,
  legs:           WrappedInventory<1>,
  feet:           WrappedInventory<1>,
  crafting:       WrappedInventory<5>,
  main:           WrappedInventory<27>,
  hotbar:         WrappedInventory<9>,
  // An index into the hotbar (0..=8)
  selected_index: u8,
  // Open window and held item
  window:         Option<WrappedInventory<27>>,
  // Held item. Always present, as survival inventories don't count as windows.
  held:           Stack,

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
      head:           WrappedInventory::new(Inventory::new(), conn.clone(), 0, 0),
      chest:          WrappedInventory::new(Inventory::new(), conn.clone(), 0, 1),
      legs:           WrappedInventory::new(Inventory::new(), conn.clone(), 0, 2),
      feet:           WrappedInventory::new(Inventory::new(), conn.clone(), 0, 3),
      crafting:       WrappedInventory::new(Inventory::new(), conn.clone(), 0, 4),
      main:           WrappedInventory::new(Inventory::new(), conn.clone(), 0, 9),
      hotbar:         WrappedInventory::new(Inventory::new(), conn, 0, 36),
      selected_index: 0,
      window:         None,
      held:           Stack::empty(),
      drag_slots:     vec![],
      player:         weak,
    }
  }

  pub fn open_window(&mut self, inv: Inventory<27>) {
    assert!(self.window.is_none());
    // Assume chest-like for now.
    self.main.offset = inv.size();
    self.main.wid = 1;
    self.hotbar.offset = inv.size() + self.main.size();
    self.hotbar.wid = 1;
    self.window = Some(WrappedInventory::new(inv, self.main.conn.clone(), 1, 0));
  }
  pub fn close_window(&mut self) {
    self.window.take();
    self.main.offset = 9;
    self.main.wid = 0;
    self.hotbar.offset = 36;
    self.hotbar.wid = 0;
  }

  /// Gives an item to the player. Returns the number of remaining items in the
  /// stack.
  pub fn give(&mut self, mut stack: Stack) -> u8 {
    let remaining = self.hotbar_mut().add(&stack);
    stack.set_amount(remaining);
    self.main_mut().add(&stack)
  }

  /// Returns the item in the player's main hand.
  pub fn main_hand(&self) -> &Stack { self.hotbar().get_raw(self.selected_index as u32).unwrap() }

  /// Returns the currently selected hotbar index. Can be used with
  /// [`hotbar`](Self::hotbar) and `get_raw` to get the item player is holding.
  /// [`main_hand`](Self::main_hand) will do the same thing.
  pub fn selected_index(&self) -> u8 { self.selected_index }

  /// Sets the selected index. Should only be used when recieving a held item
  /// slot packet.
  ///
  /// This will send equipment updates.
  pub(crate) fn set_selected(&mut self, index: u8) {
    if self.main.get(index as u32 + 36) != self.main.get(self.selected_index as u32 + 36) {
      let p = self.player.upgrade().unwrap();
      p.send_to_in_view(cb::Packet::EntityEquipment {
        eid:  p.eid(),
        slot: cb::EquipmentSlot::Hand(Hand::Main),
        item: self.main.get(index as u32 + 36).unwrap().to_item(),
      });
    }
    self.selected_index = index;
  }

  pub fn main(&self) -> &WrappedInventory<27> { &self.main }
  pub fn main_mut(&mut self) -> &mut WrappedInventory<27> { &mut self.main }

  pub fn hotbar(&self) -> &WrappedInventory<9> { &self.hotbar }
  pub fn hotbar_mut(&mut self) -> &mut WrappedInventory<9> { &mut self.hotbar }

  pub fn win(&self) -> Option<&WrappedInventory<27>> { self.window.as_ref() }
  pub fn win_mut(&mut self) -> Option<&mut WrappedInventory<27>> { self.window.as_mut() }

  /// Gets the item out of the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn get(&self, index: i32) -> Option<&Stack> {
    if index == -999 {
      return Some(&self.held);
    }
    let idx = index as u32;
    if let Some(win) = &self.window {
      match index {
        0..=26 => win.get(idx),
        27..=53 => self.main.get(idx),
        54..=62 => self.hotbar.get(idx),
        _ => None,
      }
    } else {
      match index {
        0 => self.head.get(idx),
        1 => self.chest.get(idx),
        2 => self.legs.get(idx),
        3 => self.feet.get(idx),
        4..=8 => self.crafting.get(idx),
        9..=35 => self.main.get(idx),
        36..=44 => self.hotbar.get(idx),
        _ => None,
      }
    }
  }
  // This is private as modifying the stack doesn't send an update to the client.
  pub(crate) fn get_mut(&mut self, index: i32) -> Option<&mut Stack> {
    if index == -999 {
      return Some(&mut self.held);
    }
    let idx = index as u32;
    if let Some(win) = &mut self.window {
      match index {
        0..=26 => win.get_mut(idx),
        27..=53 => self.main.get_mut(idx),
        54..=62 => self.hotbar.get_mut(idx),
        _ => None,
      }
    } else {
      match index {
        0 => self.head.get_mut(idx),
        1 => self.chest.get_mut(idx),
        2 => self.legs.get_mut(idx),
        3 => self.feet.get_mut(idx),
        4..=8 => self.crafting.get_mut(idx),
        9..=35 => self.main.get_mut(idx),
        36..=44 => self.hotbar.get_mut(idx),
        _ => None,
      }
    }
  }
  /// Replaces the item at `index` with the given item. The old item will be
  /// returned. This allows you to replace items without cloning them.
  pub fn replace(&mut self, index: i32, stack: Stack) -> Stack {
    let res = mem::replace(self.get_mut(index).unwrap(), stack);
    self.sync(index);
    res
  }
  /// Sets the item in the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn set(&mut self, index: i32, stack: Stack) {
    *self.get_mut(index).unwrap() = stack;
    self.sync(index);
  }

  /// Sends an inventory update to the client. This is more efficient than
  /// calling [`sync`](Self::sync) for all the slots in the inventory, but is
  /// less efficient than syncing a single slot. Only use this when needed, as
  /// it will send the data for every item to the client.
  pub fn sync_all(&self) {
    let mut items = vec![];
    let held = self.held.to_item();
    if let Some(inv) = &self.window {
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
    if index == self.selected_index as i32 + 36 {
      let p = self.player.upgrade().unwrap();
      p.send_to_in_view(cb::Packet::EntityEquipment {
        eid:  p.eid(),
        slot: cb::EquipmentSlot::Hand(Hand::Main),
        item: self.get(index).unwrap().to_item(),
      });
    }
    if index == -999 {
      self.main.conn.send(cb::Packet::WindowItem {
        wid:  u8::MAX,
        slot: -1,
        item: self.held.to_item(),
      });
      return;
    }
    let idx = index as u32;
    if let Some(win) = &self.window {
      match index {
        0..=26 => win.sync(idx),
        27..=53 => self.main.sync(idx),
        54..=62 => self.hotbar.sync(idx),
        _ => panic!(),
      }
    } else {
      match index {
        0 => self.head.sync(idx),
        1 => self.chest.sync(idx),
        2 => self.legs.sync(idx),
        3 => self.feet.sync(idx),
        4..=8 => self.crafting.sync(idx),
        9..=35 => self.main.sync(idx),
        36..=44 => self.hotbar.sync(idx),
        _ => panic!(),
      }
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
            Button::Right => {
              if allow {
                if self.held.is_empty() {
                  self.split(slot, -999);
                } else {
                  let it = self.get(slot).unwrap();
                  if it.item() == item::Type::Air {
                    let amount = self.held.amount();
                    self.held.set_amount(amount - 1);
                    self.set(slot, self.held.clone().with_amount(1));
                  } else if self.held.item() == it.item() {
                    let amount = self.held.amount();
                    self.held.set_amount(amount - 1);
                    let it = self.get_mut(slot).unwrap();
                    it.set_amount(it.amount() + 1);
                    self.sync(slot);
                  } else {
                    self.swap(slot, -999);
                  }
                }
              } else {
                self.sync(slot);
                self.sync(-999);
              }
            }
            Button::Middle => {}
          }
        }
      }
      ClickWindow::ShiftClick(_) => {
        if allow {
          let idx = slot as u32;
          if let Some(win) = &mut self.window {
            if let Some(mut stack) = win.get(idx).cloned() {
              let remaining = self.hotbar.add(&stack);
              stack.set_amount(remaining);
              if stack.amount() > 0 {
                let remaining = self.main.add(&stack);
                stack.set_amount(remaining);
              }
              win.set(idx, stack);
            } else if let Some(mut stack) = self.main.get(idx).cloned() {
              let remaining = win.add(&stack);
              stack.set_amount(remaining);
              self.main.set(slot as u32, stack);
            } else if let Some(mut stack) = self.hotbar.get(idx).cloned() {
              let remaining = win.add(&stack);
              stack.set_amount(remaining);
              self.main.set(idx, stack);
            }
          } else {
            // TODO: Armor slots
            if let Some(mut stack) = self.main.get(idx).cloned() {
              let remaining = self.hotbar.add(&stack);
              stack.set_amount(remaining);
              self.main.set(idx, stack);
            } else if let Some(mut stack) = self.hotbar.get(idx).cloned() {
              let remaining = self.main.add(&stack);
              stack.set_amount(remaining);
              self.hotbar.set(idx, stack);
            }
          }
        } else {
          // TODO: Only sync needed
          self.sync_all();
        }
      }
      ClickWindow::Number(num) => allow!(self.swap(slot, num as i32 + self.hotbar.offset as i32)),
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
    if self.get(b).unwrap().is_empty() {
      let mut stack = self.get(a).unwrap().clone();
      let total = stack.amount();
      stack.set_amount(total / 2);
      let remaining = total - stack.amount();
      self.set(a, stack);
      self.set(b, self.get(a).unwrap().clone().with_amount(remaining));
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
    let it = self.get(slot).unwrap();
    if it.is_empty() || it.amount() == 0 {
      return;
    }
    let removed = it.clone().with_amount(1);
    if it.amount() == 1 {
      self.replace(slot, Stack::empty());
    } else {
      let it = self.get_mut(slot).unwrap();
      it.set_amount(it.amount() - 1);
      self.sync(slot);
    }
    if let Some(p) = self.player.upgrade() {
      Self::spawn_dropped_item(&p, &removed);
    }
  }

  /// Removes the entire stack at the given slot.
  pub fn drop_all(&mut self, slot: i32) {
    let removed = self.replace(slot, Stack::empty());
    if let Some(p) = self.player.upgrade() {
      Self::spawn_dropped_item(&p, &removed);
    }
  }

  fn spawn_dropped_item(p: &Player, it: &Stack) {
    let mut meta = Metadata::new();
    meta.set_item(8, it.to_item());
    let eid = p.world().summon_meta(entity::Type::Item, p.pos() + FPos::new(0.0, 1.5, 0.0), meta);
    if let Some(e) = p.world().entities().get(eid) {
      e.set_vel(p.look_as_vec() * 0.5);
    }
  }

  /// Grabs up to a full stack of whatever item is at the given slot, and moves
  /// it to the cursor slot.
  pub fn double_click(&mut self, _slot: i32) {
    let mut held = self.replace(-999, Stack::empty());
    let start = if self.win().is_some() { 0 } else { 9 };
    let end = if let Some(win) = self.win() { win.size() + 36 } else { 45 };
    for i in start..end {
      let i = i as i32;
      let stack = self.get(i).unwrap();
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
    // TODO: Make sure we don't need this
    self.sync_all();
  }

  pub fn drag_start(&mut self) { self.drag_slots.clear(); }
  pub fn drag_add(&mut self, slot: i32) { self.drag_slots.push(slot); }
  pub fn drag_end(&mut self) {
    let stack = self.get(-999).unwrap().clone();
    let items_per_slot = stack.amount() / self.drag_slots.len() as u8;
    let items_remaining = stack.amount() % self.drag_slots.len() as u8;
    for slot in self.drag_slots.clone() {
      self.set(slot, stack.clone().with_amount(items_per_slot));
    }
    self.drag_slots.clear();
    if items_remaining == 0 {
      self.set(-999, Stack::empty());
    } else {
      let stack = self.get_mut(-999).unwrap();
      stack.set_amount(items_remaining);
      self.sync(-999);
    }
  }
}
