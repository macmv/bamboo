use tokio::net::tcp::OwnedReadHalf;

pub struct TCPReader {}

impl TCPReader {
  pub fn new(read: OwnedReadHalf) -> Self {
    TCPReader {}
  }
}
