use crate::world::WorldManager;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use mio::{
  net::{TcpListener, TcpStream},
  Events, Interest, Poll, Token,
};
use std::{
  collections::HashMap,
  io,
  io::{BufRead, Cursor, Read, Write},
  net::SocketAddr,
  string::FromUtf8Error,
  sync::Arc,
};
use thiserror::Error;

pub struct RCon {
  addr: SocketAddr,
  wm:   Arc<WorldManager>,
}
pub struct Conn<'a> {
  wm: &'a Arc<WorldManager>,

  stream:   TcpStream,
  incoming: Vec<u8>,
  outgoing: Vec<u8>,
}

impl RCon {
  pub fn new(wm: Arc<WorldManager>) -> Option<Self> {
    let config = wm.config().section("rcon");
    if !config.get::<_, bool>("enabled") {
      return None;
    }
    let addr = match config.get::<_, &str>("addr").parse() {
      Ok(a) => a,
      Err(e) => {
        error!("invalid rcon address: {e}");
        return None;
      }
    };

    Some(RCon { addr, wm })
  }

  pub fn run(&mut self) {
    let mut listen = match TcpListener::bind(self.addr) {
      Ok(l) => l,
      Err(e) => {
        error!("couldn't bind to rcon addr {}: {}", self.addr, e);
        return;
      }
    };

    const LISTEN: Token = Token(0xffffffff);

    let mut poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(128);

    poll.registry().register(&mut listen, LISTEN, Interest::READABLE).unwrap();

    let mut conns = HashMap::new();
    let mut next_token = 0;

    loop {
      poll.poll(&mut events, None).unwrap();
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
                error!("error listening in rcon: {e}");
                return;
              }
            };

            let token = Token(next_token);
            next_token += 1;
            poll
              .registry()
              .register(&mut conn, token, Interest::READABLE | Interest::WRITABLE)
              .unwrap();

            conns.insert(token, Conn::new(conn, &self.wm));
          },
          token => {
            if let Some(conn) = conns.get_mut(&token) {
              if conn.handle(event.is_readable(), event.is_writable()) {
                conns.remove(&token);
              }
            }
          }
        }
      }
    }
  }
}

enum ReadResult {
  Normal,
  Close,
}

struct Packet {
  id:   i32,
  ty:   PacketType,
  body: String,
}
enum PacketType {
  Login,
  Command,
  Output,
}

#[derive(Error, Debug)]
enum ParseError {
  #[error("invalid packet type {0}")]
  InvalidType(i32),
  #[error("cannot handle output packet")]
  CannotHandleOutput,
  #[error("invalid packet length")]
  InvalidLength,
  #[error("{0}")]
  IO(#[from] io::Error),
  #[error("{0}")]
  InvalidMessage(#[from] FromUtf8Error),
}

impl<'a> Conn<'a> {
  pub fn new(stream: TcpStream, wm: &'a Arc<WorldManager>) -> Self {
    Conn { wm, stream, incoming: vec![], outgoing: vec![] }
  }
  pub fn handle(&mut self, readable: bool, writeable: bool) -> bool {
    if readable {
      loop {
        match self.read_bytes() {
          Ok(()) => {}
          Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
          Err(e) => {
            error!("could not read packet: {e}");
            return true;
          }
        }
      }
      match self.read_packets() {
        Ok(ReadResult::Normal) => {}
        Ok(ReadResult::Close) => return true,
        Err(e) => {
          error!("could not read packet: {e}");
          return true;
        }
      }
    }
    if writeable {
      loop {
        match self.write_bytes() {
          Ok(()) => {}
          Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
          Err(e) => {
            error!("could not write packet: {e}");
            return true;
          }
        }
      }
    }
    false
  }

  fn read_bytes(&mut self) -> io::Result<()> {
    let mut buf = [0; 1024];
    let n = self.stream.read(&mut buf)?;
    self.incoming.extend_from_slice(&buf[..n]);
    Ok(())
  }
  fn write_bytes(&mut self) -> io::Result<()> {
    let n = self.stream.write(&self.outgoing)?;
    self.outgoing.drain(0..n);
    Ok(())
  }

  fn read_packets(&mut self) -> Result<ReadResult, ParseError> {
    loop {
      let p = match self.read_packet()? {
        Some(p) => p,
        None => return Ok(ReadResult::Normal),
      };
      match p.ty {
        PacketType::Login => {
          todo!();
        }
        PacketType::Command => {
          todo!();
        }
        PacketType::Output => return Err(ParseError::CannotHandleOutput),
      }
    }
  }

  fn read_packet(&mut self) -> Result<Option<Packet>, ParseError> {
    if self.incoming.len() < 4 {
      return Ok(None);
    }
    let mut buf = Cursor::new(&self.incoming);
    let len = buf.read_i32::<LittleEndian>()? + 4;
    if len < 0 || len > 4096 {
      return Err(ParseError::InvalidLength);
    }
    let id = buf.read_i32::<LittleEndian>()?;
    let ty = buf.read_i32::<LittleEndian>()?;
    let mut payload = vec![];
    let _ = buf.read_until(b'\0', &mut payload)?;
    payload.pop();
    if buf.read_u8()? != 0 {
      return Err(
        io::Error::new(io::ErrorKind::InvalidData, "expected terminating NUL byte").into(),
      );
    }
    // as u64 is fine, because we know len >= 0 (from the check above)
    if buf.position() != len as u64 {
      return Err(ParseError::InvalidLength);
    }
    self.incoming.drain(0..len as usize);
    let ty = match ty {
      3 => PacketType::Login,
      2 => PacketType::Command,
      0 => PacketType::Output,
      _ => return Err(ParseError::InvalidType(ty)),
    };
    Ok(Some(Packet { id, ty, body: String::from_utf8(payload)? }))
  }
}
