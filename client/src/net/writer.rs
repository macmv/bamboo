use tokio::net::tcp::OwnedWriteHalf;

pub struct TCPWriter {}

impl TCPWriter {
  pub fn new(write: OwnedWriteHalf) -> Self {
    TCPWriter {}
  }
}
