use common::util::Buffer;
use std::io::Cursor;

#[derive(Debug)]
pub struct Packet {
  data: Buffer<Vec<u8>>,
}

impl Packet {
  pub fn new(data: Vec<u8>) -> Packet {
    Packet { data: Buffer::new(data) }
  }
}
