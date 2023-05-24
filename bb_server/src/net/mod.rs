use crate::{player::Player, world::WorldManager};
use bb_common::{
  net::{cb, sb},
  util::{JoinInfo, ThreadPool},
  version::ProtocolVersion,
};
use bb_transfer::{
  InvalidReadError, MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError,
};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use mio::{
  event::Event,
  net::{TcpListener, TcpStream},
  Events, Interest, Poll, Token, Waker,
};
use parking_lot::{Mutex, RwLock};
use std::{
  collections::HashMap,
  convert::TryInto,
  fmt, io,
  io::{Read, Write},
  net::SocketAddr,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};

pub mod packet;
pub(crate) mod serialize;

#[cfg(test)]
mod tests;

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

#[derive(Debug, Clone)]
pub struct ConnSender {
  tx:    Sender<cb::Packet>,
  wake:  Sender<WakeEvent>,
  waker: Arc<Waker>,
  tok:   Token,
}

#[derive(Debug, Clone)]
struct EventWrapper {
  pub is_readable: bool,
  pub is_writable: bool,
}

pub struct NewConn {
  pub sender: ConnSender,
  pub info:   JoinInfo,
}

impl fmt::Debug for Connection {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("Connection").field("closed", &self.closed).finish()
  }
}

impl EventWrapper {
  pub fn new(e: &Event) -> Self {
    EventWrapper { is_readable: e.is_readable(), is_writable: e.is_writable() }
  }
}

impl ConnSender {
  #[cfg(test)]
  pub(crate) fn mock(poll: &Poll) -> (Receiver<cb::Packet>, Receiver<WakeEvent>, Self) {
    const WAKE: Token = Token(0xfffffffe);

    let (tx, rx) = crossbeam_channel::bounded(2048);
    let (wake_tx, wake_rx) = crossbeam_channel::bounded(2048);
    let waker = Arc::new(Waker::new(poll.registry(), WAKE).unwrap());
    (rx, wake_rx, ConnSender { tx, wake: wake_tx, waker, tok: Token(0) })
  }
  /// Sends the given packet to the client. Assuming there aren't too many
  /// packets in the queue, this is a non-blocking operation. This will block if
  /// there are too many packets queued. The limit is 512 packets before this
  /// will block, so this should very rarely happen.
  ///
  /// Note that this will simply drop the packet if the client has disconnected.
  ///
  /// # Panics
  ///
  /// This will panic if the waker thread used globally has been closed. The
  /// only way for this to close is if the network manager stops working.
  pub fn send(&self, p: impl Into<cb::Packet>) {
    if let Ok(()) = self.tx.send(p.into()) {
      self.wake.send(WakeEvent::Clientbound(self.tok)).unwrap();
      self.waker.wake().unwrap();
    }
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
  /// Note that this will simply drop the packet if the client has disconnected.
  ///
  /// # Panics
  ///
  /// This will panic if the waker thread used globally has been closed. The
  /// only way for this to close is if the network manager stops working.
  pub fn send(&self, p: cb::Packet) {
    if let Ok(()) = self.tx.send(p) {
      self.wake.send(WakeEvent::Clientbound(self.tok)).unwrap();
      self.waker.wake().unwrap();
    }
  }

  /// If this returns Ok(true) or an error, the connection should be closed.
  /// Ok(false) is normal operation. This will never return Err(WouldBlock).
  ///
  /// The second value in the tuple is for initialization. If a Some(player) is
  /// returned, then the next time this functions is called, that same player
  /// should be passed in. This function should be called again after
  /// Some(player) is returned, as it may not have read all availible data.
  pub fn read(&mut self) -> io::Result<(bool, Option<NewConn>, Vec<sb::Packet>)> {
    let mut out = vec![];
    loop {
      let n = match self.stream.read(&mut self.garbage) {
        Ok(0) => return Ok((true, None, out)),
        Ok(n) => n,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok((false, None, out)),
        Err(e) => return Err(e),
      };
      self.incoming.extend_from_slice(&self.garbage[..n]);
      let (new_conn, packets) = self.read_incoming()?;
      if new_conn.is_some() {
        return Ok((false, new_conn, packets));
      }
      out.extend(packets);
    }
  }

  fn try_send(&mut self) -> io::Result<()> {
    loop {
      match self.rx.try_recv() {
        Ok(p) => self.send_to_client(p)?,
        Err(TryRecvError::Empty) => break,
        Err(_e) => unreachable!(),
      }
    }
    Ok(())
  }

  fn send_to_client(&mut self, p: cb::Packet) -> io::Result<()> {
    let mut m = MessageWriter::new(self.garbage.as_mut_slice());
    p.write(&mut m).unwrap();
    let len = m.index();

    let mut prefix = [0; 5];
    let mut m = MessageWriter::new(prefix.as_mut_slice());
    m.write_u32(len.try_into().unwrap()).unwrap();
    let prefix_len = m.index();

    self.outgoing.extend_from_slice(&prefix[..prefix_len]);
    self.outgoing.extend_from_slice(&self.garbage[..len]);
    self.try_flush()
  }

  fn try_flush(&mut self) -> io::Result<()> {
    while !self.outgoing.is_empty() {
      let n = match self.stream.write(&self.outgoing) {
        Ok(v) => v,
        Err(e) => return Err(e),
      };
      self.outgoing.drain(0..n);
    }
    Ok(())
  }

  fn read_incoming(&mut self) -> io::Result<(Option<NewConn>, Vec<sb::Packet>)> {
    let mut out = vec![];
    while !self.incoming.is_empty() {
      let mut m = MessageReader::new(&self.incoming);
      match m.read_u32() {
        Ok(len) => {
          let len = len as usize;
          if len + m.index() <= self.incoming.len() {
            // Remove the length varint at the start
            let idx = m.index();
            self.incoming.drain(0..idx);
            // We already handshaked
            if self.ver.is_some() {
              let mut m = MessageReader::new(&self.incoming[..len]);
              let p = sb::Packet::read(&mut m).map_err(|err| {
                io::Error::new(
                  io::ErrorKind::InvalidData,
                  format!("while reading packet got err: {err}"),
                )
              })?;
              let n = m.index();
              self.incoming.drain(0..n);
              if n != len {
                return Err(io::Error::new(
                  io::ErrorKind::InvalidData,
                  format!("packet did not parse enough bytes (expected {len}, only parsed {n})"),
                ));
              }
              out.push(p);
            } else {
              // This is the first packet, so it must be a login packet.
              let mut m = MessageReader::new(&self.incoming[..len]);
              let info: JoinInfo = m.read().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("error reading handshake: {e}"))
              })?;
              let n = m.index();
              self.incoming.drain(0..n);
              if n != len {
                return Err(io::Error::new(
                  io::ErrorKind::InvalidData,
                  format!("handshake did not parse enough bytes (expected {len}, only parsed {n})"),
                ));
              }
              self.ver = Some(ProtocolVersion::from(info.ver as i32));
              // We rely on the caller to set the player using this value.
              return Ok((Some(NewConn { sender: self.sender(), info }), out));
            }
          } else {
            break;
          }
        }
        // If this is an EOF, then we have a partial varint, so we are done reading.
        Err(e) => {
          if matches!(e, ReadError::Invalid(InvalidReadError::EOF)) {
            return Ok((None, out));
          } else {
            return Err(io::Error::new(
              io::ErrorKind::InvalidData,
              format!("error reading packet id: {e}"),
            ));
          }
        }
      }
    }
    Ok((None, out))
  }

  // This waits for the a login packet from the proxy. If any other packet is
  // received, this will panic. This should only be called right after a
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

  // Returns true if the connection has been closed.
  pub fn closed(&self) -> bool { self.closed.load(Ordering::SeqCst) }
}

pub struct ConnectionManager {
  connections: Arc<RwLock<HashMap<Token, ConnPlayer>>>,
  wm:          Arc<WorldManager>,
}

pub enum WakeEvent {
  Clientbound(Token),
}

struct ConnPlayer {
  pub conn:   Mutex<Connection>,
  pub player: Option<Arc<Player>>,
}

struct State {
  wm:    Arc<WorldManager>,
  conns: Arc<RwLock<HashMap<Token, ConnPlayer>>>,
}

impl ConnPlayer {
  pub fn new(conn: Connection) -> Self { ConnPlayer { conn: Mutex::new(conn), player: None } }
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

    let write_pool = ThreadPool::auto("network writer", || State {
      wm:    self.wm.clone(),
      conns: self.connections.clone(),
    });
    let read_pool = ThreadPool::auto("network reader", || State {
      wm:    self.wm.clone(),
      conns: self.connections.clone(),
    });

    loop {
      loop {
        match poll.poll(&mut events, None) {
          Ok(()) => break,
          Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
          Err(e) => return Err(e),
        }
      }

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
              ConnPlayer::new(Connection::new(conn, tx.clone(), waker.clone(), token)),
            );
          },
          WAKE => {
            let r = rx.clone();
            write_pool.execute(move |s| loop {
              match r.try_recv() {
                Ok(ev) => Self::wake_event(s, ev),
                Err(TryRecvError::Empty) => break,
                Err(_) => unreachable!(),
              }
            });
          }
          token => {
            let e = EventWrapper::new(event);
            read_pool.execute(move |s| {
              if Self::handle(&s.wm, &s.conns, token, e) {
                let mut wl = s.conns.write();
                // Multiple threads can handle this event, so if the token has alrady been
                // removed, we know it was another thread that called this. Therefore, we can
                // just ignore a player that is not present.
                if let Some(p) = wl.remove(&token) {
                  drop(wl);
                  Self::handle_disconnect(&p.player);
                }
              }
            });
          }
        }
      }
    }
  }

  /// Logs a disconnect, and if the player is present, it removes them.
  fn handle_disconnect(player: &Option<Arc<Player>>) {
    if let Some(p) = player {
      p.remove();
    } else {
      info!("a client who has not finished logging in has left the game");
    }
  }

  /// If this is not a normal disconnect, then this logs an error, and calls
  /// [`disconnect_player`](Self::disconnect_player).
  fn handle_error(e: io::Error, player: &Option<Arc<Player>>) {
    if !matches!(e.kind(), io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset) {
      error!("error in connection: {}", e);
    }
    Self::handle_disconnect(player);
  }

  fn wake_event(s: &State, ev: WakeEvent) {
    match ev {
      WakeEvent::Clientbound(tok) => {
        let mut remove = false;
        if let Some(player) = s.conns.read().get(&tok) {
          remove = match player.conn.lock().try_send() {
            Ok(()) => false,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => false,
            Err(e) => {
              Self::handle_error(e, &player.player);
              true
            }
          };
        }
        if remove {
          s.conns.write().remove(&tok);
        }
      }
    }
  }

  fn handle(
    wm: &Arc<WorldManager>,
    c: &RwLock<HashMap<Token, ConnPlayer>>,
    token: Token,
    ev: EventWrapper,
  ) -> bool {
    if ev.is_readable {
      loop {
        let rl = c.read();
        // If this isn't present, we assume another thread has removed the player, and
        // we return.
        if let Some(player) = rl.get(&token) {
          // Make sure we drop conn! We can get a deadlock if we call `packet::handle`
          // when this is locked.
          let (disconnect, new_conn, packets) = match player.conn.lock().read() {
            Ok(v) => v,
            // Something else went wrong.
            Err(e) => {
              Self::handle_error(e, &player.player);
              return true;
            }
          };
          if disconnect {
            Self::handle_disconnect(&player.player);
            return true;
          }
          // Don't drop our read lock yet, as we need to use the player we got from it.
          if let Some(player) = &player.player {
            if packets.is_empty() {
              break;
            }
            for p in packets {
              packet::handle(wm, player, p);
            }
          } else {
            drop(rl);
            // The player must be created after we drop the `conn.lock()`, so that sending
            // login packets doesn't deadlock.
            if let Some(new_conn) = new_conn {
              let new_player = wm.new_player(new_conn.sender, new_conn.info);
              {
                let mut wl = c.write();
                let player: &mut ConnPlayer = wl.get_mut(&token).unwrap();
                player.player = Some(new_player);
              }
              let rl = c.read();
              if let Some(player) = rl.get(&token) {
                for p in packets {
                  packet::handle(wm, player.player.as_ref().unwrap(), p);
                }
              }
            }
          }
        } else {
          // We return false because the player has already been removed from the map.
          return false;
        }
      }
    }
    if ev.is_writable {
      let rl = c.read();
      if let Some(player) = rl.get(&token) {
        let mut conn = player.conn.lock();
        match conn.try_flush() {
          Ok(()) => {}
          Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
          Err(e) => {
            Self::handle_error(e, &player.player);
            return true;
          }
        }
      } else {
        // We return false because the player has already been removed from the map.
        return false;
      }
    }
    false
  }
}
