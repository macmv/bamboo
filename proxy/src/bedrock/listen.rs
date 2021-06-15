use super::{BedrockStreamReader, BedrockStreamWriter};
use ringbuf::{Producer, RingBuffer};
use std::{collections::HashMap, io, net::SocketAddr, sync::Arc};
use tokio::net::UdpSocket;

pub struct Listener {
  sock:    Arc<UdpSocket>,
  clients: HashMap<SocketAddr, Producer<u8>>,
}

impl Listener {
  pub async fn bind<A: Into<String>>(addr: A) -> io::Result<Self> {
    Ok(Listener { sock: Arc::new(UdpSocket::bind(addr.into()).await?), clients: HashMap::new() })
  }
  pub async fn poll(&mut self) -> io::Result<Option<(BedrockStreamReader, BedrockStreamWriter)>> {
    let mut buf = vec![0; 256];
    let (len, src) = self.sock.recv_from(&mut buf).await?;
    if let Some(prod) = self.clients.get_mut(&src) {
      // Got data from a client that already exists
      prod.push_slice(&buf[..len]);
      Ok(None)
    } else {
      // New client
      let buf = RingBuffer::new(1024);
      let (prod, cons) = buf.split();

      let reader = BedrockStreamReader::new(cons);
      let writer = BedrockStreamWriter::new(self.sock.clone(), src);
      self.clients.insert(src, prod);
      info!("got new client {}", src);
      Ok(Some((reader, writer)))
    }
  }
}
