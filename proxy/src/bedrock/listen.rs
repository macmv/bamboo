use super::{StreamReader, StreamWriter};
use std::{io, net::UdpSocket, sync::Arc};

pub struct Listener {
  sock: Arc<UdpSocket>,
}

impl Listener {
  pub fn bind<A: Into<String>>(addr: A) -> io::Result<Self> {
    Ok(Listener { sock: Arc::new(UdpSocket::bind(addr.into())?) })
  }
  pub fn accept(&self) -> io::Result<(StreamReader, StreamWriter)> {
    // Wait for a client to
    self.sock.peek_from(&mut vec![0])?;
    Ok((StreamReader::new(self.sock.clone()), StreamWriter::new(self.sock.clone())))
  }
}
