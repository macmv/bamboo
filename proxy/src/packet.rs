use common::util::Buffer;

#[derive(Debug)]
pub struct Packet {
  data: Buffer,
}

impl Packet {
  pub fn new(data: Vec<u8>) -> Packet {
    Packet { data: Buffer::new(data) }
  }
}
