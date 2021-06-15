use super::{BedrockStreamReader, BedrockStreamWriter};
use std::{
  collections::HashMap,
  io,
  net::{SocketAddr, UdpSocket},
  sync::{
    mpsc::{self, Sender},
    Arc,
  },
};

pub struct Listener {
  sock:    Arc<UdpSocket>,
  clients: HashMap<SocketAddr, Sender<Vec<u8>>>,
}

impl Listener {
  pub fn bind<A: Into<String>>(addr: A) -> io::Result<Self> {
    Ok(Listener { sock: Arc::new(UdpSocket::bind(addr.into())?), clients: HashMap::new() })
  }
  pub async fn poll(&mut self) -> io::Result<Option<(BedrockStreamReader, BedrockStreamWriter)>> {
    let mut buf = vec![0; 256];
    let (len, src) = self.sock.recv_from(&mut buf)?;
    if let Some(tx) = self.clients.get(&src) {
      // Got data from a client that already exists
      tx.send(buf[..len].to_vec());
      Ok(None)
    } else {
      // New client
      let (tx, rx) = mpsc::channel();
      let reader = BedrockStreamReader::new(rx);
      let writer = BedrockStreamWriter::new(self.sock.clone(), src);
      self.clients.insert(src, tx);
      Ok(Some((reader, writer)))
    }
  }
}
