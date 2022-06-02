use bb_common::{math::der, util::UUID, version::ProtocolVersion};
use bb_proxy::{
  conn::State,
  gnet::{cb, sb, tcp},
  stream::{java::JavaStream, PacketStream},
  Result,
};
use rand::{rngs::OsRng, Rng};
use rsa::PublicKey;
use std::io;

pub struct ConnStream {
  stream: JavaStream,
  ver:    ProtocolVersion,
  closed: bool,
  state:  State,
}

impl ConnStream {
  pub fn new(stream: JavaStream) -> Self {
    ConnStream { stream, ver: ProtocolVersion::V1_8, closed: false, state: State::Handshake }
  }
  pub fn start_handshake(&mut self) {
    let mut out = tcp::Packet::new(0, self.ver);
    out.write_varint(self.ver.id() as i32);
    out.write_str("127.0.0.1");
    out.write_u16(25565);
    out.write_varint(2); // login state
    self.stream.write(out);
    self.state = State::Login;
    let mut out = tcp::Packet::new(0, self.ver);
    out.write_str("macmv");
    self.stream.write(out);
  }
  pub fn write(&mut self, p: sb::Packet) {
    let mut tcp = tcp::Packet::new(p.tcp_id(self.ver) as i32, self.ver);
    p.to_tcp(&mut tcp);
    self.stream.write(tcp);
  }
  pub fn needs_flush(&self) -> bool { self.stream.needs_flush() }
  pub fn flush(&mut self) -> Result<()> { self.stream.flush() }
  pub fn closed(&self) -> bool { self.closed }

  pub fn poll(&mut self) -> Result<()> { self.stream.poll() }
  pub fn read(&mut self) -> Result<Option<cb::Packet>> {
    if let Some(mut p) = self.stream.read(self.ver)? {
      match self.state {
        State::Play => Ok(Some(cb::Packet::from_tcp(&mut p, self.ver)?)),
        _ => {
          self.handle_handshake(p)?;
          Ok(None)
        }
      }
    } else {
      Ok(None)
    }
  }

  fn handle_handshake(&mut self, mut p: tcp::Packet) -> Result<()> {
    match self.state {
      State::Handshake => unreachable!(),
      State::Status => unreachable!(),
      State::Login => match p.id() {
        0 => {
          // disconnect
          let reason = p.read_ident()?;
          error!("disconnected: {}", reason);
          self.closed = true;
          return Ok(());
        }
        1 => {
          // encryption request
          warn!("got encryption request, but mojang auth is not implemented");

          let _server_id = p.read_ident()?;
          let pub_key_len = p.read_varint()?;
          let pub_key = p.read_buf(pub_key_len.try_into().unwrap())?;
          let token_len = p.read_varint()?;
          let token = p.read_buf(token_len.try_into().unwrap())?;
          let key = der::decode(&pub_key).unwrap();

          let mut secret = [0; 16];
          let mut rng = OsRng;
          rng.fill(&mut secret);

          let enc_secret = key
            .encrypt(&mut rng, rsa::PaddingScheme::PKCS1v15Encrypt, &secret)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
          let enc_token = key
            .encrypt(&mut rng, rsa::PaddingScheme::PKCS1v15Encrypt, &token)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

          let mut out = tcp::Packet::new(1, ProtocolVersion::V1_8);
          out.write_varint(enc_secret.len() as i32);
          out.write_buf(&enc_secret);
          out.write_varint(enc_token.len() as i32);
          out.write_buf(&enc_token);
          self.stream.write(out);
          self.stream.enable_encryption(&secret);
        }
        2 => {
          // login success
          let _uuid = if self.ver >= ProtocolVersion::V1_16 {
            p.read_uuid()?
          } else {
            UUID::from_dashed_str(&p.read_str(36)?)
              .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?
          };
          let _username = p.read_str(16)?;
          self.state = State::Play;
        }
        3 => {
          // set compression
          let thresh = p.read_varint()?;
          self.stream.set_compression(thresh);
        }
        _ => unreachable!(),
      },
      State::Play => unreachable!(),
      State::Invalid => unreachable!(),
    }
    Ok(())
  }
}
