use super::{BedrockStreamReader, BedrockStreamWriter};
use std::{
  collections::HashMap,
  io,
  net::{SocketAddr, UdpSocket},
  sync::Arc,
};

pub struct Listener {
  sock:    Arc<UdpSocket>,
  clients: HashMap<SocketAddr, BedrockStreamReader>,
}

impl Listener {
  pub fn bind<A: Into<String>>(addr: A) -> io::Result<Self> {
    Ok(Listener { sock: Arc::new(UdpSocket::bind(addr.into())?), clients: HashMap::new() })
  }
  pub async fn poll(&self) -> io::Result<Option<(BedrockStreamReader, BedrockStreamWriter)>> {
    let mut buf = vec![0; 256];
    let (len, src) = self.sock.recv_from(&mut buf)?;
    if let Some(reader) = self.clients.get(&src) {
      // Got data from a client that already exists
      reader.append(buf[..len]);
      Ok(None)
    } else {
      // New client
      let reader = BedrockStreamReader::new(self.sock.clone(), src);
      let writer = BedrockStreamWriter::new(self.sock.clone(), src);
      self.clients.insert(src, reader);
      Ok(Some((reader, writer)))
    }
  }
}
