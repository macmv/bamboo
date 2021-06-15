use super::{BedrockStreamReader, BedrockStreamWriter};
use std::{io, net::UdpSocket, sync::Arc};

pub struct Listener {
  sock: Arc<UdpSocket>,
}

impl Listener {
  pub fn bind<A: Into<String>>(addr: A) -> io::Result<Self> {
    Ok(Listener { sock: Arc::new(UdpSocket::bind(addr.into())?) })
  }
  pub fn accept(&self) -> io::Result<(BedrockStreamReader, BedrockStreamWriter)> {
    // Wait for a client to
    self.sock.peek_from(&mut vec![0])?;
    Ok((BedrockStreamReader::new(self.sock.clone()), BedrockStreamWriter::new(self.sock.clone())))
  }
}
