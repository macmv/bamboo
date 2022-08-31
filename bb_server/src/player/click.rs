use super::{Player, PlayerInventory};
use crate::{
  block,
  block::Block,
  event,
  math::{CollisionResult, Vec3},
};
use bb_common::{
  math::{FPos, Pos},
  util::{Chat, Face, GameMode},
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct BlockClick<'a> {
  /// The player that click on a block.
  pub player: &'a Arc<Player>,
  /// The direction the player was looking when they clicked.
  pub dir:    Vec3,
  /// The block that was clicked on.
  pub block:  Block<'a>,
  /// The face that was clicked on. For example, `Face::Top` means the player
  /// was looking at the top face of a block.
  pub face:   Face,
  /// The position within the face. Relative to the minimum corner, so this will
  /// be within 0 - 1 on all axis.
  pub cursor: FPos,
}
#[derive(Debug, Clone, Copy)]
pub struct AirClick<'a> {
  pub player: &'a Arc<Player>,
  pub dir:    Vec3,
}

#[derive(Debug, Clone, Copy)]
pub enum Click<'a> {
  Air(AirClick<'a>),
  Block(BlockClick<'a>),
}

impl Click<'_> {
  pub fn do_raycast(&self, distance: f64, water: bool) -> Option<(FPos, CollisionResult)> {
    let from = self.player().pos() + FPos::new(0.0, self.player().eyes_offset(), 0.0);
    let to = from + self.dir() * distance;

    self.player().world().raycast(from, to, water)
  }

  pub fn player(&self) -> &Arc<Player> {
    match self {
      Self::Air(air) => air.player,
      Self::Block(block) => block.player,
    }
  }
  pub fn dir(&self) -> Vec3 {
    match self {
      Self::Air(air) => air.dir,
      Self::Block(block) => block.dir,
    }
  }
}

impl BlockClick<'_> {
  /// Places the given block as the player. This will use `block.pos` as the
  /// `clicked_pos`, and the given position as `placed_pos`.
  ///
  /// This is intended for item handlers which override block placements, and
  /// need to place a seperate kind of block.
  pub fn place(&self, pos: Pos, ty: block::Type) {
    self.player.place_block(self.block.pos, pos, ty);
  }
}

impl Player {
  /// Places a block as the given player. This will send a block place event,
  /// remove an item from their hotbar, and send a hotbar message if the
  /// position is outside of the world.
  pub fn place_block(self: &Arc<Player>, clicked_pos: Pos, placed_pos: Pos, ty: block::Type) {
    self.place_block_lock(&mut self.lock_inventory(), clicked_pos, placed_pos, ty);
  }

  /// Same as [`place_block`](Self::place_block), but this uses an inventory
  /// that has already been locked. This should be preferred, as it means the
  /// item in the player's hotbar won't change between the lookup and the block
  /// place.
  pub fn place_block_lock(
    self: &Arc<Player>,
    inv: &mut PlayerInventory,
    clicked_pos: Pos,
    placed_pos: Pos,
    ty: block::Type,
  ) {
    if self
      .world()
      .events()
      .player_request(event::BlockPlace {
        player: self.clone(),
        clicked_pos,
        placed_pos,
        block: ty.to_store(),
      })
      .is_handled()
    {
      self.sync_block_at(placed_pos);
      inv.sync_main_hand();
      return;
    }

    match self.world().set_block(placed_pos, ty) {
      Ok(_) => {
        if self.game_mode() != GameMode::Creative {
          let idx = inv.selected_index() as u32;
          let stack = inv.hotbar_mut().get_raw_mut(idx).unwrap();
          // Don't overflow if they just placed a block while holding nothing.
          if stack.amount() >= 1 {
            stack.set_amount(stack.amount() - 1);
            inv.hotbar().sync_raw(idx);
          }
        }
      }
      Err(e) => {
        self.send_hotbar(Chat::new(e.to_string()));
        self.sync_block_at(placed_pos);
      }
    }
  }
}
