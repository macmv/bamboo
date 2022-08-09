use super::Window;
use crate::{
  entity, event, item,
  item::{SingleInventory, Stack},
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
  util::{GameMode, Hand},
};
use std::{mem, sync::Weak};

#[derive(Debug)]
pub struct PlayerInventory {
  head:           SingleInventory<1>,
  chest:          SingleInventory<1>,
  legs:           SingleInventory<1>,
  feet:           SingleInventory<1>,
  crafting:       SingleInventory<5>,
  main:           SingleInventory<27>,
  hotbar:         SingleInventory<9>,
  off_hand:       SingleInventory<1>,
  // An index into the hotbar (0..=8)
  selected_index: u8,
  // Open window and held item
  window:         Option<Window>,
  // Held item. Always present, as survival inventories don't count as windows.
  held:           Stack,

  /// Used when draging the mouse over items. This is sent as a collection of
  /// packets all at once, after the drag is finished. Because it's sent as
  /// multiple packets, we need to hold onto this state.
  drag_slots: Vec<i32>,
  /// Used for validation that the client is being consistent. If they change
  /// buttons part way through the drag, we silently ignore the whole thing.
  drag_bt:    Option<Button>,

  player: Weak<Player>,
}

impl PlayerInventory {
  pub fn new(weak: Weak<Player>, conn: ConnSender) -> Self {
    // We always store an inventory with 46 slots, even if the client is on 1.8 (in
    // that version, there was no off-hand).
    PlayerInventory {
      head:           SingleInventory::new(conn.clone(), 0, 0),
      chest:          SingleInventory::new(conn.clone(), 0, 1),
      legs:           SingleInventory::new(conn.clone(), 0, 2),
      feet:           SingleInventory::new(conn.clone(), 0, 3),
      crafting:       SingleInventory::new(conn.clone(), 0, 4),
      main:           SingleInventory::new(conn.clone(), 0, 9),
      hotbar:         SingleInventory::new(conn.clone(), 0, 36),
      off_hand:       SingleInventory::new(conn, 0, 45),
      selected_index: 0,
      window:         None,
      held:           Stack::empty(),
      drag_slots:     vec![],
      drag_bt:        None,
      player:         weak,
    }
  }

  pub fn open_window(&mut self, win: Window) {
    assert!(self.window.is_none());
    // Assume chest-like for now.
    self.main.offset = win.size();
    self.main.wid = 1;
    self.hotbar.offset = win.size() + self.main.size();
    self.hotbar.wid = 1;
    let p = self.player.upgrade().unwrap();
    win.open(p.uuid, &p.conn);
    self.window = Some(win);
  }
  pub fn close_window(&mut self) {
    if let Some(win) = self.window.take() {
      let p = self.player.upgrade().unwrap();
      win.close(p.uuid);
    }
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
  /// Returns the item in the player's off hand.
  pub fn off_hand(&self) -> &Stack { self.off_hand.get_raw(0).unwrap() }
  /// Returns the item in the given hand.
  pub fn in_hand(&self, hand: Hand) -> &Stack {
    match hand {
      Hand::Main => self.main_hand(),
      Hand::Off => self.off_hand(),
    }
  }

  /// Returns the currently selected hotbar index. Can be used with
  /// [`hotbar`](Self::hotbar) and `get_raw` to get the item player is holding.
  /// [`main_hand`](Self::main_hand) will do the same thing.
  pub fn selected_index(&self) -> u8 { self.selected_index }

  /// Sets the selected index. Should only be used when receiving a held item
  /// slot packet.
  ///
  /// This will send equipment updates.
  pub(crate) fn set_selected(&mut self, index: u8) {
    if self.hotbar.get_raw(index as u32) != self.hotbar.get_raw(self.selected_index as u32) {
      let p = self.player.upgrade().unwrap();
      p.send_to_in_view(cb::packet::EntityEquipment {
        eid:  p.eid(),
        slot: cb::EquipmentSlot::Hand(Hand::Main),
        item: self.hotbar.get_raw(index as u32).unwrap().to_item(),
      });
    }
    self.selected_index = index;
  }

  pub fn main(&self) -> &SingleInventory<27> { &self.main }
  pub fn main_mut(&mut self) -> &mut SingleInventory<27> { &mut self.main }

  pub fn hotbar(&self) -> &SingleInventory<9> { &self.hotbar }
  pub fn hotbar_mut(&mut self) -> &mut SingleInventory<9> { &mut self.hotbar }

  pub fn win(&self) -> Option<&Window> { self.window.as_ref() }
  pub fn win_mut(&mut self) -> Option<&mut Window> { self.window.as_mut() }

  /// Gets the item out of the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn get(&self, index: i32) -> Option<Stack> {
    if index == -999 {
      return Some(self.held.clone());
    }
    let idx = index as u32;
    if let Some(win) = &self.window {
      if idx < win.size() {
        win.get(idx)
      } else if idx < win.size() + 27 {
        self.main.get(idx).cloned()
      } else if idx < win.size() + 36 {
        self.hotbar.get(idx).cloned()
      } else {
        None
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
        45 => self.off_hand.get(idx),
        _ => None,
      }
      .cloned()
    }
  }
  // This is private as modifying the stack doesn't send an update to the client.
  pub(crate) fn access<R>(&mut self, index: i32, f: impl FnOnce(&mut Stack) -> R) -> Option<R> {
    if index == -999 {
      return Some(f(&mut self.held));
    }
    let idx = index as u32;
    if let Some(win) = &mut self.window {
      if idx < win.size() {
        win.access_mut(idx, f)
      } else if idx < win.size() + 27 {
        self.main.get_mut(idx).map(f)
      } else if idx < win.size() + 36 {
        self.hotbar.get_mut(idx).map(f)
      } else {
        None
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
        45 => self.off_hand.get_mut(idx),
        _ => None,
      }
      .map(f)
    }
  }
  /// Replaces the item at `index` with the given item. The old item will be
  /// returned. This allows you to replace items without cloning them.
  pub fn replace(&mut self, index: i32, stack: Stack) -> Stack {
    let res = self.access(index, move |it| mem::replace(it, stack)).unwrap();
    self.sync(index);
    res
  }
  /// Sets the item in the inventory. This uses absolute ids, so depending
  /// on if a window is open, the actual slot being accessed may change. Use
  /// [`main`](Self::main) or [`win`](Self::win) to access the main inventory or
  /// the open window directly.
  pub fn set(&mut self, index: i32, stack: Stack) {
    self.access(index, |it| *it = stack);
    self.sync(index);
  }

  /// Sends an inventory update to the client. This is more efficient than
  /// calling [`sync`](Self::sync) for all the slots in the inventory, but is
  /// less efficient than syncing a single slot. Only use this when needed, as
  /// it will send the data for every item to the client.
  pub fn sync_all(&self) {
    let mut items = vec![];
    if let Some(win) = &self.window {
      for it in win.items() {
        items.push(it.to_item());
      }
    } else {
      for it in self.head.inv.items() {
        items.push(it.to_item());
      }
      for it in self.chest.inv.items() {
        items.push(it.to_item());
      }
      for it in self.legs.inv.items() {
        items.push(it.to_item());
      }
      for it in self.feet.inv.items() {
        items.push(it.to_item());
      }
      for it in self.crafting.inv.items() {
        items.push(it.to_item());
      }
    }
    for it in self.main.inv.items().iter() {
      items.push(it.to_item());
    }
    for it in self.hotbar.inv.items() {
      items.push(it.to_item());
    }
    let held = self.held.to_item();
    self.main.conn.send(cb::packet::WindowItems { wid: 1, items, held });
  }
  /// Sends an item update for the given slot. This shouldn't every be needed,
  /// as functions like [`set`](Self::set) and [`replace`](Self::replace) will
  /// call this for you.
  pub fn sync(&self, index: i32) {
    if index == self.selected_index as i32 + 36 {
      let p = self.player.upgrade().unwrap();
      p.send_to_in_view(cb::packet::EntityEquipment {
        eid:  p.eid(),
        slot: cb::EquipmentSlot::Hand(Hand::Main),
        item: self.get(index).unwrap().to_item(),
      });
    }
    if index == -999 {
      self.main.conn.send(cb::packet::WindowItem {
        wid:  u8::MAX,
        slot: -1,
        item: self.held.to_item(),
      });
      return;
    }
    let idx = index as u32;
    if let Some(win) = &self.window {
      if idx < win.size() {
        win.sync(idx);
      } else if idx < win.size() + 27 {
        self.main.sync(idx);
      } else if idx < win.size() + 36 {
        self.hotbar.sync(idx);
      } else {
        panic!()
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
        45 => self.off_hand.sync(idx),
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
            Button::Left => {
              if allow {
                let mut it = self.get(slot).unwrap();
                if it.item() == self.held.item() {
                  // Merge stacks in `slot`
                  if it.amount() + self.held.amount() > 64 {
                    self.held.set_amount((it.amount() + self.held.amount()) - 64);
                    it.set_amount(64);
                  } else {
                    it.set_amount(it.amount() + self.held.amount());
                    self.held.set_amount(0);
                  }
                  self.set(slot, it);
                  self.sync(-999);
                } else {
                  self.swap(slot, -999);
                }
              } else {
                self.sync(slot);
                self.sync(-999);
              }
            }
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
                    if self.get(slot).unwrap().amount() < 64 {
                      let amount = self.held.amount();
                      self.held.set_amount(amount - 1);
                      self.access(slot, |s| s.set_amount(s.amount() + 1));
                      self.sync(slot);
                    }
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
            if let Some(mut stack) = win.get(idx) {
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
              self.hotbar.set(idx, stack);
              self.sync_all();
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
      ClickWindow::DragStart(bt) => self.drag_start(bt),
      ClickWindow::DragAdd(bt) => self.drag_add(bt, slot),
      ClickWindow::DragEnd(bt) => self.drag_end(bt),
    }

    self.sync_all();
  }

  /// Takes half of the items in the slot `a` and moves them to `b`. If `b` is
  /// not empty, this is a noop.
  pub fn split(&mut self, a: i32, b: i32) {
    if self.get(b).unwrap().is_empty() {
      let stack = self.get(a).unwrap();
      let total = stack.amount();
      if total == 1 {
        // Edge case. Avoids cloning `a` when we don't need to.
        self.swap(a, b);
      } else {
        let a_amt = total / 2;
        let b_amt = total - (total / 2);
        self.access(a, |a| a.set_amount(a_amt));
        self.sync(a);
        self.set(b, stack.with_amount(b_amt));
      }
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
    if let Some(p) = self.player.upgrade() {
      if p
        .world
        .events()
        .player_request(event::ItemDrop {
          player:     p.clone(),
          stack:      it.clone(),
          full_stack: false,
        })
        .is_handled()
      {
        self.sync(slot);
        return;
      }
    }
    let removed = it.clone().with_amount(1);
    if it.amount() == 1 {
      self.replace(slot, Stack::empty());
    } else {
      self.access(slot, |s| s.set_amount(s.amount() - 1));
      self.sync(slot);
    }
    if let Some(p) = self.player.upgrade() {
      Self::spawn_dropped_item(&p, &removed);
    }
  }

  /// Removes the entire stack at the given slot.
  pub fn drop_all(&mut self, slot: i32) {
    let it = self.get(slot).unwrap();
    if it.is_empty() || it.amount() == 0 {
      return;
    }
    if let Some(p) = self.player.upgrade() {
      if p
        .world
        .events()
        .player_request(event::ItemDrop { player: p.clone(), stack: it, full_stack: true })
        .is_handled()
      {
        self.sync(slot);
        return;
      }
    }
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
  pub fn double_click(&mut self, slot: i32) {
    if let Some(p) = self.player.upgrade() {
      if p
        .world()
        .events()
        .player_request(event::InvDoubleClick { player: p.clone(), slot })
        .is_handled()
      {
        return;
      }
    }
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

  pub fn drag_start(&mut self, bt: Button) {
    self.drag_slots.clear();
    self.drag_bt = Some(bt);
  }
  pub fn drag_add(&mut self, bt: Button, slot: i32) {
    if self.drag_bt != Some(bt) {
      self.drag_bt = None;
      self.drag_slots.clear();
      return;
    }
    self.drag_slots.push(slot);
  }
  pub fn drag_end(&mut self, bt: Button) {
    if self.drag_bt.take() != Some(bt) {
      self.drag_slots.clear();
      return;
    }
    if self.drag_slots.is_empty() {
      return;
    }
    let stack = self.get(-999).unwrap();
    let items_per_slot;
    let mut items_remaining;
    match bt {
      Button::Left => {
        items_per_slot = stack.amount() / self.drag_slots.len() as u8;
        items_remaining = stack.amount() % self.drag_slots.len() as u8;
      }
      Button::Right => {
        items_per_slot = 1;
        while self.drag_slots.len() as u8 > stack.amount() {
          self.drag_slots.pop();
        }
        items_remaining = stack.amount() - self.drag_slots.len() as u8;
      }
      Button::Middle => {
        // Older clients still send this when they aren't supposed to.
        if self.player.upgrade().unwrap().game_mode() != GameMode::Creative {
          self.sync_all();
          self.drag_slots.clear();
          return;
        }
        items_per_slot = 64;
        items_remaining = stack.amount();
      }
    }
    for slot in self.drag_slots.clone() {
      self.access(slot, |s| {
        if s.item() == item::Type::Air {
          *s = stack.clone().with_amount(items_per_slot);
        } else if s.item() == stack.item() {
          // Same item. Here, we add to the slot, and put any overflow in
          // `items_remaining`.
          if s.amount() + items_per_slot > 64 {
            items_remaining += 64 - s.amount();
            s.set_amount(64);
          } else {
            s.set_amount(s.amount() + items_per_slot);
          }
        } else {
          // Different item, so we just put this slots amount in `items_remaining`.
          //
          // Note that if this slot has a different type, the client won't send this
          // slot. However, if they do, we just handle it like this, to avoid losing
          // items.
          items_remaining += items_per_slot;
        }
      });
      self.sync(slot);
    }
    self.drag_slots.clear();
    self.access(-999, |stack| stack.set_amount(items_remaining));
    self.sync(-999);
  }
}
