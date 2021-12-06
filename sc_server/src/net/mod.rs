use crate::{block, entity, item, player::Player, world::WorldManager};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use mio::{
  event::Event,
  net::{TcpListener, TcpStream},
  Events, Interest, Poll, Token, Waker,
};
use parking_lot::{Mutex, RwLock};
use sc_common::{
  math::{FPos, Pos},
  net::{cb, sb},
  util::{
    chat::{Chat, Color, HoverEvent},
    ThreadPool, UUID,
  },
  version::ProtocolVersion,
};
use sc_transfer::{MessageReader, MessageWriter, ReadError};
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
  ///
  /// # Panics
  ///
  /// This has the possibility to panic if any of the channels this uses are
  /// disconnected. This will not happen unless the connection has been closed,
  /// or the connection manager has been stopped.
  pub fn send(&self, p: cb::Packet) {
    self.tx.send(p).unwrap();
    self.wake.send(WakeEvent::Clientbound(self.tok)).unwrap();
    self.waker.wake().unwrap();
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
  ///
  /// # Panics
  ///
  /// This has the possibility to panic if any of the channels this uses are
  /// disconnected. This will not happen unless the connection has been closed,
  /// or the connection manager has been stopped.
  pub fn send(&self, p: cb::Packet) {
    self.tx.send(p).unwrap();
    self.wake.send(WakeEvent::Clientbound(self.tok)).unwrap();
    self.waker.wake().unwrap();
  }

  /// If this returns Ok(true) or an error, the connection should be closed.
  /// Ok(false) is normal operation. This will never return Err(WouldBlock).
  ///
  /// The second value in the tuple is for initialization. If a Some(player) is
  /// returned, then the next time this functions is called, that same player
  /// should be passed in. This function should be called again after
  /// Some(player) is returned, as it may not have read all availible data.
  pub fn read(
    &mut self,
    wm: &Arc<WorldManager>,
    player: &Option<Arc<Player>>,
  ) -> io::Result<(bool, Option<Arc<Player>>)> {
    loop {
      let n = match self.stream.read(&mut self.garbage) {
        Ok(0) => return Ok((true, None)),
        Ok(n) => n,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok((false, None)),
        Err(e) => return Err(e),
      };
      self.incoming.extend_from_slice(&self.garbage[..n]);
      if let Some(p) = self.read_incoming(wm, player)? {
        return Ok((false, Some(p)));
      }
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
    let mut m = MessageWriter::new(&mut self.garbage);
    p.to_sc(&mut m).unwrap();
    let len = m.index();

    let mut prefix = [0; 5];
    let mut m = MessageWriter::new(&mut prefix);
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
    player: &Option<Arc<Player>>,
  ) -> io::Result<Option<Arc<Player>>> {
    while !self.incoming.is_empty() {
      let mut m = MessageReader::new(&self.incoming);
      match m.read_u32() {
        Ok(len) => {
          if len as usize + m.index() <= self.incoming.len() {
            // Remove the length varint at the start
            let idx = m.index();
            self.incoming.drain(0..idx);
            // We already handshaked
            if let Some(ver) = self.ver {
              let mut m = MessageReader::new(&self.incoming);
              let p = sb::Packet::from_sc(&mut m, ver).map_err(|err| {
                io::Error::new(
                  io::ErrorKind::InvalidData,
                  format!("while reading packet got err: {}", err),
                )
              })?;
              info!("got packet: {:?}", p);
              let n = m.index();
              if n != len as usize {
                return Err(io::Error::new(
                  io::ErrorKind::InvalidData,
                  format!(
                    "packet did not parse enough bytes (expected {}, only parsed {})",
                    len, n
                  ),
                ));
              }
              self.incoming.drain(0..n);
              self.handle_packet(wm, player.as_ref().unwrap(), p);
            } else {
              // This is the first packet, so it must be a login packet.
              let mut m = MessageReader::new(&self.incoming);
              let username = m.read_str().map_err(|e| {
                io::Error::new(
                  io::ErrorKind::InvalidData,
                  format!("error reading handshake: {}", e),
                )
              })?;
              let uuid = UUID::from_be_bytes(
                m.read_bytes(16)
                  .map_err(|e| {
                    io::Error::new(
                      io::ErrorKind::InvalidData,
                      format!("error reading handshake: {}", e),
                    )
                  })?
                  .try_into()
                  .unwrap(),
              );
              let ver = ProtocolVersion::from(m.read_i32().map_err(|e| {
                io::Error::new(
                  io::ErrorKind::InvalidData,
                  format!("error reading handshake: {}", e),
                )
              })?);
              let idx = m.index();
              self.incoming.drain(0..idx);
              self.ver = Some(ver);
              // We rely on the caller to set the player using this value.
              return Ok(Some(wm.new_player(self.sender(), username, uuid, ver)));
            }
          } else {
            break;
          }
        }
        // If this is an EOF, then we have a partial varint, so we are done reading.
        Err(e) => {
          if matches!(e, ReadError::EOF) {
            return Ok(None);
          } else {
            return Err(io::Error::new(
              io::ErrorKind::InvalidData,
              format!("error reading packet id: {}", e),
            ));
          }
        }
      }
    }
    Ok(None)
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
      /*
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
      sb::Packet::UseItem { hand_v1_9 } => {
        // 0 = main hand on 1.8
        let hand = hand_v1_9.unwrap_or(0);
        self.use_item(player, hand);
      }
      sb::Packet::BlockPlace {
        mut location,
        direction_v1_8,
        direction_v1_9,
        hand_v1_9,
        cursor_x_v1_8: _,
        cursor_x_v1_11: _,
        cursor_y_v1_8: _,
        cursor_y_v1_11: _,
        cursor_z_v1_8: _,
        cursor_z_v1_11: _,
        inside_block_v1_14: _,
        held_item_removed_v1_9: _,
      } => {
        // 0 = main hand on 1.8
        let hand = hand_v1_9.unwrap_or(0);

        let direction: i32 = if player.ver() == ProtocolVersion::V1_8 {
          // direction_v1_8 is an i8 (not a u8), so the sign stays correct
          direction_v1_8.unwrap().into()
        } else {
          direction_v1_9.unwrap()
        };

        if location == Pos::new(-1, -1, -1) && direction == -1 {
          self.use_item(player, hand);
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
      */
      // _ => warn!("got unknown packet from client: {:?}", p),
      _ => (),
    }
  }

  fn use_item(&self, player: &Arc<Player>, _hand: i32) {
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

  // Returns true if the connection has been closed.
  pub fn closed(&self) -> bool {
    self.closed.load(Ordering::SeqCst)
  }
}

pub struct ConnectionManager {
  connections: Arc<RwLock<HashMap<Token, (Mutex<Connection>, Option<Arc<Player>>)>>>,
  wm:          Arc<WorldManager>,
}

pub enum WakeEvent {
  Clientbound(Token),
}

struct State {
  wm:    Arc<WorldManager>,
  conns: Arc<RwLock<HashMap<Token, (Mutex<Connection>, Option<Arc<Player>>)>>>,
}

impl ConnectionManager {
  pub fn new(wm: Arc<WorldManager>) -> ConnectionManager {
    ConnectionManager { connections: Arc::new(RwLock::new(HashMap::new())), wm }
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

    let pool =
      ThreadPool::auto(|| State { wm: self.wm.clone(), conns: self.connections.clone() });

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

            self.connections.write().insert(
              token,
              (Mutex::new(Connection::new(conn, tx.clone(), waker.clone(), token)), None),
            );
          },
          WAKE => loop {
            match rx.try_recv() {
              Ok(ev) => self.wake_event(ev),
              Err(TryRecvError::Empty) => break,
              Err(_) => unreachable!(),
            }
          },
          token => {
            let e = event.clone();
            pool.execute(move |s| {
              if Self::handle(&s.wm, &s.conns, token, e) {
                let mut c = s.conns.write();
                let (_, p) = c.remove(&token).expect("got event for a client that does not exist");
                s.wm.remove_player(p.as_ref().unwrap().id());
              }
            });
          }
        }
      }
    }
  }

  fn wake_event(&mut self, ev: WakeEvent) {
    match ev {
      WakeEvent::Clientbound(tok) => {
        if let Some((conn, _)) = self.connections.read().get(&tok) {
          match conn.lock().try_send() {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => {
              error!("error in connection: {}", e);
              self.connections.write().remove(&tok);
            }
          }
        }
      }
    }
  }

  fn handle(
    wm: &Arc<WorldManager>,
    c: &RwLock<HashMap<Token, (Mutex<Connection>, Option<Arc<Player>>)>>,
    token: Token,
    ev: Event,
  ) -> bool {
    if ev.is_readable() {
      loop {
        let rl = c.read();
        let (conn, player) = rl.get(&token).expect("got event for a client that does not exist");
        let mut conn = conn.lock();
        match conn.read(&wm, player) {
          Ok((false, Some(p))) => {
            // The handshake was just completed, so now we need to add the player into the
            // main hashmap. So we drop the read lock, and then lock the map for writing.
            // This is the only situation where we don't break, as conn.read() will return
            // early if it gets a player.
            drop(conn);
            drop(rl);
            let mut wl = c.write();
            let (_, player) = wl.get_mut(&token).unwrap();
            *player = Some(p);
          }
          // Normal operation. We are done reading all the data.
          Ok((false, None)) => break,
          // Connection is closed without an error.
          Ok((true, _)) => return true,
          // Something else went wrong.
          Err(e) => {
            error!("error in connection: {}", e);
            return true;
          }
        }
      }
    }
    if ev.is_writable() {
      let rl = c.read();
      let (conn, _) = rl.get(&token).expect("got event for a client that does not exist");
      let mut conn = conn.lock();
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
