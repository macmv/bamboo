use crate::{block, item, player::Player, world::WorldManager};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use mio::{
  event::Event,
  net::{TcpListener, TcpStream},
  Events, Interest, Poll, Registry, Token, Waker,
};
use sc_common::{
  math::Pos,
  net::{cb, sb},
  util::{
    chat::{Chat, Color, HoverEvent},
    UUID,
  },
  version::ProtocolVersion,
};
use sc_transfer::{MessageRead, MessageWrite, ReadError};
use std::{
  collections::HashMap,
  convert::TryInto,
  io,
  io::{Read, Write},
  net::SocketAddr,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};

pub(crate) mod serialize;

pub struct Connection {
  stream: TcpStream,
  ver:    Option<ProtocolVersion>,
  closed: AtomicBool,

  /// Sending on this will send a packet to the client.
  tx:    Sender<cb::Packet>,
  rx:    Receiver<cb::Packet>,
  wake:  Sender<WakeEvent>,
  waker: Arc<Waker>,
  tok:   Token,

  incoming: Vec<u8>,
  outgoing: Vec<u8>,
  garbage:  Vec<u8>,
}

pub struct ConnSender {
  tx:    Sender<cb::Packet>,
  wake:  Sender<WakeEvent>,
  waker: Arc<Waker>,
  tok:   Token,
}

impl ConnSender {
  /// Sends the given packet to the client. Assuming there aren't too many
  /// packets in the queue, this is a non-blocking operation. This will block if
  /// there are too many packets queued. The limit is 512 packets before this
  /// will block, so this should very rarely happen.
  pub fn send(&self, p: cb::Packet) {
    self.tx.send(p);
    self.wake.send(WakeEvent::Clientbound(self.tok));
    self.waker.wake();
  }
}

impl Connection {
  pub(crate) fn new(
    stream: TcpStream,
    wake: Sender<WakeEvent>,
    waker: Arc<Waker>,
    tok: Token,
  ) -> Self {
    // For a 10 chunk render distance, we need to send 441 packets at once. So a
    // limit of 512 means we don't block very much.
    let (tx, rx) = crossbeam_channel::bounded(512);
    Connection {
      stream,
      ver: None,
      closed: false.into(),
      tx,
      rx,
      wake,
      waker,
      tok,
      incoming: Vec::with_capacity(1024),
      outgoing: Vec::with_capacity(1024),
      garbage: vec![0; 256 * 1024],
    }
  }

  /// Creates a sender that will send packets to the client on this connection.
  /// This needs to clone a few arcs, so it should not be used frequently.
  pub fn sender(&self) -> ConnSender {
    ConnSender {
      tx:    self.tx.clone(),
      wake:  self.wake.clone(),
      waker: self.waker.clone(),
      tok:   self.tok,
    }
  }

  /// Sends the given packet to the client. Assuming there aren't too many
  /// packets in the queue, this is a non-blocking operation. This will block if
  /// there are too many packets queued. The limit is 512 packets before this
  /// will block, so this should very rarely happen.
  pub fn send(&self, p: cb::Packet) {
    self.tx.send(p);
    self.wake.send(WakeEvent::Clientbound(self.tok));
    self.waker.wake();
  }

  /// This will return ErrorKind::WouldBlock once its done reading. If it
  /// returns any other error, the connection should be closed.
  pub fn read(&mut self, wm: &Arc<WorldManager>, player: &mut Option<Arc<Player>>) -> io::Error {
    loop {
      let n = match self.stream.read(&mut self.garbage) {
        Ok(v) => v,
        Err(e) => return e,
      };
      self.incoming.extend_from_slice(&self.garbage[..n]);
      match self.read_incoming(wm, player) {
        Ok(_) => {}
        Err(e) => return io::Error::new(io::ErrorKind::InvalidData, e.to_string()),
      };
    }
  }

  fn try_send(&mut self) -> io::Result<()> {
    loop {
      match self.rx.try_recv() {
        Ok(p) => self.send_to_client(p)?,
        Err(TryRecvError::Empty) => break,
        Err(e) => unreachable!(),
      }
    }
    Ok(())
  }

  fn send_to_client(&mut self, p: cb::Packet) -> io::Result<()> {
    let len = p.to_sc(self.ver.unwrap(), &mut self.garbage).unwrap();

    let mut prefix = [0; 5];
    let mut m = MessageWrite::new(&mut prefix);
    m.write_i32(len as i32).unwrap();
    let prefix_len = m.index();

    self.outgoing.extend_from_slice(&prefix[..prefix_len]);
    self.outgoing.extend_from_slice(&self.garbage[..len]);
    self.try_flush()
  }

  fn try_flush(&mut self) -> io::Result<()> {
    while !self.outgoing.is_empty() {
      let n = match self.stream.write(&mut self.outgoing) {
        Ok(v) => v,
        Err(e) => return Err(e),
      };
      self.outgoing.drain(0..n);
    }
    Ok(())
  }

  fn read_incoming(
    &mut self,
    wm: &Arc<WorldManager>,
    player: &mut Option<Arc<Player>>,
  ) -> Result<(), ReadError> {
    while !self.incoming.is_empty() {
      let mut m = MessageRead::new(&self.incoming);
      match m.read_i32() {
        Ok(len) => {
          if len as usize + m.index() <= self.incoming.len() {
            // Remove the length varint at the start
            let idx = m.index();
            self.incoming.drain(0..idx);
            // We already handshaked
            if let Some(ver) = self.ver {
              let (p, n) = sb::Packet::from_sc(ver, &self.incoming)?;
              if n != len as usize {
                return Err(ReadError::EOF);
              }
              self.incoming.drain(0..n);
              self.handle_packet(wm, player.as_ref().unwrap(), p);
            } else {
              // This is the first packet, so it must be a login packet.
              let mut m = MessageRead::new(&self.incoming);
              let username = m.read_str()?;
              let uuid = UUID::from_bytes(m.read_bytes(16)?.try_into().unwrap());
              let ver = ProtocolVersion::from(m.read_i32()?);
              let idx = m.index();
              self.incoming.drain(0..idx);
              self.ver = Some(ver);
              *player = Some(wm.new_player(self.sender(), username, uuid, ver));
            }
          } else {
            break;
          }
        }
        // If this is an EOF, then we have a partial varint, so we are done reading.
        Err(e) => {
          if !matches!(e, ReadError::EOF) {
            return Ok(());
          } else {
            return Err(e);
          }
        }
      }
    }
    Ok(())
  }

  // This waits for the a login packet from the proxy. If any other packet is
  // recieved, this will panic. This should only be called right after a
  // connection is created.
  //
  // pub(crate) async fn wait_for_login(&mut self) -> (String, UUID,
  // ProtocolVersion) {   let p = match
  // self.rx.lock().message().unwrap() {     // This version
  // doesn't matter, as the proxy will always send the same data for every
  // version     Some(p) => sb::Packet::from_proto(p, ProtocolVersion::V1_8),
  //     None => panic!("connection was closed while listening for a login
  // packet"),   };
  //   match p {
  //     sb::Packet::Login { username, uuid, ver } => {
  //       let ver = ProtocolVersion::from(ver);
  //       self.ver = Some(ver);
  //       (username, uuid, ver)
  //     }
  //     _ => panic!("expecting login packet, got: {:?}", p),
  //   }
  // }

  /// This starts up the recieving loop for this connection. Do not call this
  /// more than once.
  pub(crate) fn handle_packet(&self, wm: &Arc<WorldManager>, player: &Arc<Player>, p: sb::Packet) {
    match p {
      sb::Packet::Chat { message } => {
        if message.chars().next() == Some('/') {
          let mut chars = message.chars();
          chars.next().unwrap();
          player.world().commands().execute(wm, player, chars.as_str());
        } else {
          let mut msg = Chat::empty();
          msg.add("<");
          msg.add(player.username()).color(Color::BrightGreen).on_hover(HoverEvent::ShowText(
            format!("wow it is almost like {} sent this message", player.username()),
          ));
          msg.add("> ");
          msg.add(message);
          player.world().broadcast(msg);
        }
      }
      sb::Packet::SetCreativeSlot { slot, item } => {
        if slot > 0 {
          let id =
            player.world().item_converter().to_latest(item.id() as u32, player.ver().block());
          player
            .lock_inventory()
            .set(slot as u32, item::Stack::new(item::Type::from_u32(id)).with_amount(item.count()));
        }
      }
      sb::Packet::BlockDig { location, status: _, face: _ } => {
        player.world().set_kind(location, block::Kind::Air).unwrap();
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
          let item_data = {
            let inv = player.lock_inventory();
            let stack = inv.main_hand();
            player.world().item_converter().get_data(stack.item())
          };
          let kind = item_data.block_to_place();

          match player.world().get_block(location) {
            Ok(looking_at) => {
              let block_data = player.world().block_converter().get(looking_at.kind());
              if !block_data.material.is_replaceable() {
                let _ = player.sync_block_at(location);
                location += Pos::dir_from_byte(direction.try_into().unwrap());
              }

              match player.world().set_kind(location, kind) {
                Ok(_) => player.world().plugins().on_block_place(player.clone(), location, kind),
                Err(e) => player.send_hotbar(&Chat::new(e.to_string())),
              }
            }
            Err(e) => player.send_hotbar(&Chat::new(e.to_string())),
          };
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
  }

  // Returns true if the connection has been closed.
  pub fn closed(&self) -> bool {
    self.closed.load(Ordering::SeqCst)
  }
}

pub struct ConnectionManager {
  connections: HashMap<Token, (Connection, Option<Arc<Player>>)>,
  new_tok:     Token,
  wm:          Arc<WorldManager>,
}

pub enum WakeEvent {
  Clientbound(Token),
}

impl ConnectionManager {
  pub fn new(wm: Arc<WorldManager>) -> ConnectionManager {
    ConnectionManager { connections: HashMap::new(), new_tok: Token(0), wm }
  }

  pub fn run(&mut self, addr: SocketAddr) -> io::Result<()> {
    const LISTEN: Token = Token(0xffffffff);
    const WAKE: Token = Token(0xfffffffe);

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);
    let mut listen = TcpListener::bind(addr)?;

    let waker = Arc::new(Waker::new(poll.registry(), WAKE)?);

    poll.registry().register(&mut listen, LISTEN, Interest::READABLE)?;

    let mut next_token = 0;

    let (tx, rx) = crossbeam_channel::bounded(1024);

    loop {
      poll.poll(&mut events, None)?;

      for event in events.iter() {
        match event.token() {
          LISTEN => loop {
            // Received an event for the TCP server socket, which
            // indicates we can accept an connection.
            let (mut conn, _addr) = match listen.accept() {
              Ok(v) => v,
              Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // If we get a `WouldBlock` error we know our
                // listener has no more incoming connections queued,
                // so we can return to polling and wait for some
                // more.
                break;
              }
              Err(e) => {
                // If it was any other kind of error, something went
                // wrong and we terminate with an error.
                return Err(e);
              }
            };

            let token = Token(next_token);
            next_token += 1;
            poll.registry().register(&mut conn, token, Interest::READABLE | Interest::WRITABLE)?;

            self
              .connections
              .insert(token, (Connection::new(conn, tx.clone(), waker.clone(), token), None));
          },
          WAKE => loop {
            match rx.try_recv() {
              Ok(ev) => self.wake_event(ev),
              Err(TryRecvError::Empty) => break,
              Err(_) => unreachable!(),
            }
          },
          token => {
            // We got an even for a tcp connection. If the token is invalid, we ignore it.
            let done = if let Some((conn, player)) = self.connections.get_mut(&token) {
              Self::handle(&self.wm, poll.registry(), conn, player, event)
            } else {
              false
            };
            if done {
              self.connections.remove(&token);
            }
          }
        }
      }
    }
  }

  fn wake_event(&mut self, ev: WakeEvent) {
    match ev {
      WakeEvent::Clientbound(tok) => {
        if let Some((conn, _)) = self.connections.get_mut(&tok) {
          match conn.try_send() {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => {
              error!("error in connection: {}", e);
              self.connections.remove(&tok);
            }
          }
        }
      }
    }
  }

  fn handle(
    wm: &Arc<WorldManager>,
    reg: &Registry,
    conn: &mut Connection,
    player: &mut Option<Arc<Player>>,
    ev: &Event,
  ) -> bool {
    if ev.is_readable() {
      let err = conn.read(&wm, player);
      match err.kind() {
        io::ErrorKind::WouldBlock => {}
        _ => {
          error!("error in connection: {}", err);
          return true;
        }
      }
    }
    if ev.is_writable() {
      match conn.try_flush() {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
        Err(e) => {
          error!("error in connection: {}", e);
          return true;
        }
      }
    }
    false
  }
}
