use crate::{
  block,
  block::Block,
  entity, item,
  player::{Click, Player},
  world::WorldManager,
};
use bb_common::{
  math::{FPos, Pos},
  net::{cb, sb},
  util::{
    chat::{Chat, Color, HoverEvent},
    GameMode,
  },
};
use std::{str::FromStr, sync::Arc};

/// Handles a single packet.
pub(crate) fn handle(wm: &Arc<WorldManager>, player: &Arc<Player>, p: sb::Packet) {
  // TODO: This depends on debug formatting, which is unstable. Also, it is slow,
  // because we allocate every time this is called.
  /*
  let log_packets = wm.config().get::<_, Vec<String>>("log-packets");
  if !log_packets.is_empty() {
    let msg = format!("{p:?}");
    for log in log_packets {
      if log == "all" || msg.starts_with(&log) {
        info!("packet: {msg}");
        break;
      }
    }
  }
  */
  match p {
    sb::Packet::KeepAlive { id: _ } => {
      // TODO Keep aliev packets
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

      if let Some(command) = msg.strip_prefix('/') {
        player.world().commands().execute(wm, player, command);
      } else {
        let text = msg;
        let mut msg = Chat::empty();
        msg.add("<");
        msg.add(player.username()).color(Color::BrightGreen).on_hover(HoverEvent::ShowText(
          format!("wow it is almost like {} sent this message", player.username()),
        ));
        msg.add("> ");
        msg.add(text);
        wm.broadcast(msg);
      }
    }
    sb::Packet::BlockDig { pos, status, face: _ } => {
      // If the world is locked then we need to sync this block.
      if player.world().is_locked() {
        player.sync_block_at(pos).unwrap();
      } else {
        match player.game_mode() {
          GameMode::Survival => match status {
            sb::DigStatus::Start => player.start_digging(pos),
            sb::DigStatus::Cancel => player.cancel_digging(),
            sb::DigStatus::Finish => player.finish_digging(pos),
          },
          GameMode::Creative => {
            // Avoid race condition
            if !player.world().set_kind(pos, block::Kind::Air).unwrap() {
              player.sync_block_at(pos).unwrap();
            }
          }
          // TODO: Not sure if the sync is needed, but it won't hurt much.
          GameMode::Adventure => {
            player.sync_block_at(pos).unwrap();
          }
          // We will just ignore block digs from spectators, as they won't show any updates client
          // side.
          GameMode::Spectator => {}
        }
      }
    }
    sb::Packet::CreativeInventoryUpdate { slot, item } => {
      if slot >= 0 {
        player.lock_inventory().set(slot.into(), item.into());
      }
    }
    sb::Packet::ClickWindow { wid, mut slot, mode } => {
      if wid == u8::MAX {
        slot = i16::from(player.lock_inventory().selected_index()) + 36;
      }
      let allow =
        player.world().plugins().on_click_window(player.clone(), slot.into(), mode.clone());
      player.lock_inventory().click_window(slot.into(), mode, allow);
    }
    sb::Packet::ChangeHeldItem { slot } => {
      player.lock_inventory().set_selected(slot);
    }
    sb::Packet::UseItem { hand: _ } => {
      // Spawn a snowball (for fun)

      /*
      let eid =
        player.world().summon(entity::Type::ArmorStand, player.pos() + FPos::new(0.0, 1.0, 0.0));

      // If the entity doesn't exist, it already despawned, so we do nothing if
      // it isn't in the world.
      if let Some(ent) = player.world().entities().get(eid) {
        ent.set_vel(player.look_as_vec() * 1.0);
      }
      */
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
        match player.world().get_block(pos) {
          Ok(looking_at) => {
            let handled = wm
              .block_behaviors()
              .call(looking_at.kind(), |b| {
                b.interact(Block::new(player.world(), pos, looking_at), player)
              })
              .unwrap_or(false);

            if handled {
              let _ = player.sync_block_at(pos);
              let _ = player.sync_block_at(pos + face);
              return;
            }

            // TODO: Data generator should store which items are blockitems, and what blocks
            // they place.
            let mut inv = player.lock_inventory();
            let stack = inv.main_hand();
            let item_data = player.world().item_converter().get_data(stack.item());
            let kind = block::Kind::from_str(item_data.name()).unwrap_or_else(|_| {
              player.send_message(Chat::new(format!("ah! {} is confusing", item_data.name())));
              block::Kind::Air
            });

            let click = Click { face, dir: player.look_as_vec() };
            let placing_data = wm.block_converter().get(kind);
            let ty = wm
              .block_behaviors()
              .call(kind, |b| b.place(placing_data, pos, click))
              .unwrap_or_else(|| placing_data.default_type());

            let looking_data = wm.block_converter().get(looking_at.kind());
            if !looking_data.material.is_replaceable() {
              let _ = player.sync_block_at(pos);
              pos += face;
            }

            match player.world().set_block(pos, ty) {
              Ok(_) => {
                if player.game_mode() != GameMode::Creative {
                  let idx = inv.selected_index() as u32;
                  let stack = inv.hotbar_mut().get_raw_mut(idx).unwrap();
                  if stack.amount() >= 1 {
                    stack.set_amount(stack.amount() - 1);
                    inv.hotbar().sync_raw(idx);
                  }
                }
                drop(inv);
                // TODO: Handle plugins cancelling this place.
                player.world().plugins().on_block_place(player.clone(), pos, ty);
              }
              Err(e) => player.send_hotbar(Chat::new(e.to_string())),
            }
          }
          Err(e) => player.send_hotbar(Chat::new(e.to_string())),
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
    sb::Packet::Flying { flying } => {
      player.set_flying_no_send(flying);
    }
    // Just contains on_ground
    sb::Packet::PlayerOnGround { .. } => {}
    sb::Packet::PlayerCommand { command } => player.handle_command(command),
    sb::Packet::Animation { hand } => player.send_to_in_view(cb::Packet::Animation {
      eid:  player.eid(),
      kind: cb::Animation::Swing(hand),
    }),
    sb::Packet::UseEntity { eid, action, sneaking } => {
      if let Some(crouching) = sneaking {
        player.set_crouching(crouching);
      }
      if let Some(ent) = player.world().entities().get(eid) {
        match action {
          sb::UseEntityAction::Attack => player.attack(ent),
          _ => warn!("todo: action {action:?}"),
        }
      }
    }
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
    if let Some(ent) = player.world().entities().get(eid) {
      ent.set_vel(player.look_as_vec() * 0.5);
    }
  }
}
