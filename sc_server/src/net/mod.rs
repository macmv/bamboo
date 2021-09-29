use std::{
  convert::TryInto,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{mpsc::Sender, Mutex};
use tonic::{Status, Streaming};

use sc_common::{
  math::Pos,
  net::{cb, sb},
  proto,
  util::{
    chat::{Chat, Color, HoverEvent},
    UUID,
  },
  version::ProtocolVersion,
};

use crate::{block, player::Player, world::WorldManager};

pub(crate) mod serialize;

pub struct Connection {
  rx:     Mutex<Streaming<proto::Packet>>,
  tx:     Sender<Result<proto::Packet, Status>>,
  ver:    Option<ProtocolVersion>,
  closed: AtomicBool,
}

impl Connection {
  pub(crate) fn new(
    rx: Streaming<proto::Packet>,
    tx: Sender<Result<proto::Packet, Status>>,
  ) -> Self {
    Connection { rx: Mutex::new(rx), tx, ver: None, closed: false.into() }
  }

  /// This waits for the a login packet from the proxy. If any other packet is
  /// recieved, this will panic. This should only be called right after a
  /// connection is created.
  pub(crate) async fn wait_for_login(&mut self) -> (String, UUID, ProtocolVersion) {
    let p = match self.rx.lock().await.message().await.unwrap() {
      // This version doesn't matter, as the proxy will always send the same data for every version
      Some(p) => sb::Packet::from_proto(p, ProtocolVersion::V1_8),
      None => panic!("connection was closed while listening for a login packet"),
    };
    match p {
      sb::Packet::Login { username, uuid, ver } => {
        let ver = ProtocolVersion::from(ver);
        self.ver = Some(ver);
        (username, uuid, ver)
      }
      _ => panic!("expecting login packet, got: {:?}", p),
    }
  }

  /// This starts up the recieving loop for this connection. Do not call this
  /// more than once.
  pub(crate) async fn run(&self, player: Arc<Player>, wm: Arc<WorldManager>) -> Result<(), Status> {
    'running: loop {
      if self.closed() {
        break 'running;
      }
      let p = match self.rx.lock().await.message().await {
        Ok(Some(p)) => sb::Packet::from_proto(p, player.ver()),
        Ok(None) => break 'running,
        Err(e) => {
          // For whatever reason, we get this unknown error every now and then. It's ugly,
          // but this is the only way to check for it.
          if e.code() == tonic::Code::Cancelled {
            break 'running;
          } else {
            return Err(e);
          }
        }
      };
      match p {
        sb::Packet::Chat { message } => {
          if message.chars().next() == Some('/') {
            let mut chars = message.chars();
            chars.next().unwrap();
            player.world().get_commands().execute(wm.clone(), player.clone(), chars.as_str()).await;
          } else {
            let mut msg = Chat::empty();
            msg.add("<");
            msg.add(player.username()).color(Color::BrightGreen).on_hover(HoverEvent::ShowText(
              format!("wow it is almost like {} sent this message", player.username()),
            ));
            msg.add("> ");
            msg.add(message);
            player.world().broadcast(msg).await;
          }
        }
        sb::Packet::SetCreativeSlot { slot, item } => {
          info!("got set creative slot at slot {} with item {:?}", slot, item);
          // TODO: Parse the item
          // let id = p.get_int("item-id");
          // let count = p.get_byte("item-count");
          // let _nbt = p.get_byte_arr("item-nbt");

          // if slot > 0 {
          //   let id = player.world().get_item_converter().to_latest(id as u32,
          // player.ver().block());   player
          //     .lock_inventory()
          //     .set(slot as u32,
          // item::Stack::new(item::Type::from_u32(id)).with_amount(count));
          // }
        }
        sb::Packet::BlockDig { location, status: _, face: _ } => {
          player.world().set_kind(location, block::Kind::Air).await.unwrap();
        }
        sb::Packet::HeldItemSlot { slot_id } => {
          player.lock_inventory().set_selected(slot_id.try_into().unwrap());
        }
        sb::Packet::BlockPlace {
          mut location,
          direction_v1_8,
          direction_v1_9,
          hand_v1_9: _,
          cursor_x_v1_8: _,
          cursor_x_v1_11: _,
          cursor_y_v1_8: _,
          cursor_y_v1_11: _,
          cursor_z_v1_8: _,
          cursor_z_v1_11: _,
          inside_block_v1_14: _,
          held_item_removed_v1_9: _,
        } => {
          let direction: i32 = if player.ver() == ProtocolVersion::V1_8 {
            // direction_v1_8 is an i8 (not a u8), so the sign stays correct
            direction_v1_8.unwrap().into()
          } else {
            direction_v1_9.unwrap()
          };

          if location == Pos::new(-1, -1, -1) && direction == -1 {
            // Client is eating, or head is inside block
          } else {
            let data = {
              let inv = player.lock_inventory();
              let stack = inv.main_hand();
              player.world().get_item_converter().get_data(stack.item())
            };
            let kind = data.block_to_place();
            location += Pos::dir_from_byte(direction.try_into().unwrap());
            match player.world().set_kind(location, kind).await {
              Ok(_) => (),
              Err(e) => player.send_hotbar(&Chat::new(e.to_string())).await,
            }
            player.world().get_plugins().on_block_place(player.clone(), location, kind);
          }
        }
        sb::Packet::Position { x, y, z, on_ground: _ } => {
          player.set_next_pos(x, y, z);
        }
        sb::Packet::PositionLook { x, y, z, yaw, pitch, on_ground: _ } => {
          player.set_next_pos(x, y, z);
          player.set_next_look(yaw, pitch);
        }
        sb::Packet::Look { yaw, pitch, on_ground: _ } => {
          player.set_next_look(yaw, pitch);
        }
        // _ => warn!("got unknown packet from client: {:?}", p),
        _ => (),
      }
      // info!("got packet from client {:?}", p);
    }
    self.closed.store(true, Ordering::SeqCst);
    Ok(())
  }

  /// Sends a packet to the proxy, which will then get sent to the client.
  pub async fn send(&self, p: cb::Packet) {
    match self.tx.send(Ok(p.to_proto(self.ver.unwrap()))).await {
      Ok(_) => (),
      Err(e) => {
        error!("error while sending packet: {}", e);
        self.closed.store(true, Ordering::SeqCst);
      }
    }
  }

  // Returns true if the connection has been closed.
  pub fn closed(&self) -> bool {
    self.closed.load(Ordering::SeqCst)
  }
}
