use crate::{block, entity, item, player::Player, world::WorldManager};
use bb_common::{
  math::{FPos, Pos},
  net::sb,
  util::chat::{Chat, Color, HoverEvent},
};
use std::{str::FromStr, sync::Arc};

/// This starts up the recieving loop for this connection. Do not call this
/// more than once.
pub(crate) fn handle(wm: &Arc<WorldManager>, player: &Arc<Player>, p: sb::Packet) {
  match p {
    sb::Packet::KeepAlive { id: _ } => {
      // TODO
    }
    sb::Packet::Chat { msg } => {
      /*
      player.lock_scoreboard().show();
      player.lock_scoreboard().set_line(1, &Chat::new("foo"));
      player.lock_scoreboard().set_line(2, &Chat::new("bar"));
      let mut c = Chat::new("foo");
      c.add(" bar").color(Color::BrightGreen);
      player.lock_scoreboard().set_line(3, &c);
      */

      if msg.chars().next() == Some('/') {
        let mut chars = msg.chars();
        chars.next().unwrap();
        player.world().commands().execute(wm, player, chars.as_str());
      } else {
        let text = msg;
        let mut msg = Chat::empty();
        msg.add("<");
        msg.add(player.username()).color(Color::BrightGreen).on_hover(HoverEvent::ShowText(
          format!("wow it is almost like {} sent this message", player.username()),
        ));
        msg.add("> ");
        msg.add(text);
        player.world().broadcast(msg);
      }
    }
    sb::Packet::BlockDig { pos, status: _, face: _ } => {
      // If the world is locked then we need to sync this block.
      if player.world().is_locked() {
        player.sync_block_at(pos).unwrap();
      } else {
        // Avoid race condition
        if !player.world().set_kind(pos, block::Kind::Air).unwrap() {
          player.sync_block_at(pos).unwrap();
        }
      }
    }
    sb::Packet::CreativeInventoryUpdate { slot, item } => {
      if slot >= 0 {
        player.lock_inventory().set(slot.into(), item.into());
      }
    }
    sb::Packet::ClickWindow { id: _, slot, mode } => {
      let allow =
        player.world().plugins().on_click_window(player.clone(), slot.into(), mode.clone());
      player.lock_inventory().click_window(slot.into(), mode, allow);
    }
    sb::Packet::ChangeHeldItem { slot } => {
      player.lock_inventory().set_selected(slot);
    }
    sb::Packet::UseItem { hand: _ } => {
      // Spawn a snowball (for fun)
      //
      // let eid = player.world().summon(entity::Type::Item, player.pos() +
      // FPos::new(0.0, 1.0, 0.0));
      //
      // If the entity doesn't exist, it already despawned, so we do nothing if
      // it isn't in the world.
      //
      // player.world().entities().get(&eid).map(|ent|
      // ent.set_vel(player.look_as_vec() * 1.0));
    }
    sb::Packet::BlockPlace { mut pos, face, hand: _ } => {
      /*
      let direction: i32 = if player.ver() == ProtocolVersion::V1_8 {
        // direction_v1_8 is an i8 (not a u8), so the sign stays correct
        direction_v1_8.unwrap().into()
      } else {
        direction_v1_9.unwrap()
      };
      */

      if pos == Pos::new(-1, -1, -1)
      /* && face == -1 */
      {
        // self.use_item(player, hand);
      } else {
        // TODO: Data generator should store which items are blockitems, and what blocks
        // they place.
        let item_data = {
          let inv = player.lock_inventory();
          let stack = inv.main_hand();
          player.world().item_converter().get_data(stack.item())
        };
        let kind = block::Kind::from_str(item_data.name()).unwrap_or_else(|_| {
          player.send_message(&Chat::new(format!("ah! {} is confusing", item_data.name())));
          block::Kind::Air
        });

        match player.world().get_block(pos) {
          Ok(looking_at) => {
            let block_data = player.world().block_converter().get(looking_at.kind());
            if !block_data.material.is_replaceable() {
              let _ = player.sync_block_at(pos);
              pos += face;
            }

            let ty = player.world().block_converter().get(kind).default_type();
            match player.world().set_block(pos, ty) {
              Ok(_) => player.world().plugins().on_block_place(player.clone(), pos, ty),
              Err(e) => player.send_hotbar(&Chat::new(e.to_string())),
            }
          }
          Err(e) => player.send_hotbar(&Chat::new(e.to_string())),
        };
      }
    }
    sb::Packet::PlayerPos { x, y, z, .. } => {
      player.set_next_pos(x, y, z);
    }
    sb::Packet::PlayerPosLook { x, y, z, yaw, pitch, .. } => {
      player.set_next_pos(x, y, z);
      player.set_next_look(yaw, pitch);
    }
    sb::Packet::PlayerLook { yaw, pitch, .. } => {
      player.set_next_look(yaw, pitch);
    }
    // Just contains on_ground
    sb::Packet::PlayerOnGround { .. } => {}
    sb::Packet::WindowClose { wid: _ } => player.lock_inventory().close_window(),
    _ => warn!("unknown packet: {:?}", p),
  }
}

#[allow(unused)]
fn use_item(player: &Arc<Player>, _hand: i32) {
  // TODO: Offhand
  let inv = player.lock_inventory();
  let main = inv.main_hand();
  if main.item() == item::Type::Snowball {
    let eid = player.world().summon(entity::Type::Slime, player.pos() + FPos::new(0.0, 1.0, 0.0));
    // If the entity doesn't exist, it already despawned, so we do nothing if it
    // isn't in the world.
    player.world().entities().get(&eid).map(|ent| ent.set_vel(player.look_as_vec() * 0.5));
  }
}
