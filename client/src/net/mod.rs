use tokio::net::TcpStream;

mod reader;
mod writer;

use reader::TCPReader;
use writer::TCPWriter;

pub struct Connection {
  reader: TCPReader,
  writer: TCPWriter,
}

impl Connection {
  pub async fn new(ip: &str) -> Option<Self> {
    info!("connecting to {}...", ip);
    let stream = match TcpStream::connect(ip).await {
      Ok(s) => s,
      Err(e) => {
        error!("could not connect to {}: {}", ip, e);
        return None;
      }
    };

    let (read, write) = stream.into_split();
    let conn = Connection { reader: TCPReader::new(read), writer: TCPWriter::new(write) };
    conn.handshake("macmv");

    Some(conn)
  }

  async fn handshake(&self, name: &str) {}
}
