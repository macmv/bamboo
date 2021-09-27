use sc_common::{
  net::{cb, sb, tcp},
  version::ProtocolVersion,
};
use sc_proxy::stream::{java::JavaStream, PacketStream};
use std::io;

pub struct ConnStream {
  stream: JavaStream,
  ver:    ProtocolVersion,
  closed: bool,
}

impl ConnStream {
  pub fn new(stream: JavaStream) -> Self {
    ConnStream { stream, ver: ProtocolVersion::V1_8, closed: false }
  }
  pub fn start_handshake(&mut self) {
    let mut out = tcp::Packet::new(0, self.ver);
    out.write_varint(self.ver.id() as i32);
    out.write_str("127.0.0.1");
    out.write_u16(25565);
    out.write_varint(2); // login state
    self.stream.write(out);
    let mut out = tcp::Packet::new(0, self.ver);
    out.write_str("macmv");
    self.stream.write(out);
  }
  pub fn write(&mut self, p: sb::Packet) {
    self.stream.write(p.to_tcp(self.ver))
  }
  pub fn needs_flush(&self) -> bool {
    self.stream.needs_flush()
  }
  pub fn flush(&mut self) -> Result<(), io::Error> {
    self.stream.flush()
  }
  pub fn closed(&self) -> bool {
    self.closed
  }

  pub fn poll(&mut self) -> Result<(), io::Error> {
    self.stream.poll()
  }
  pub fn read(&mut self) -> Result<Option<cb::Packet>, io::Error> {
    Ok(self.stream.read(self.ver)?.map(|p| cb::Packet::from_tcp(p, self.ver)))
  }
}
