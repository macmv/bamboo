use common::util::{Buffer, BufferError};

#[derive(Debug)]
pub struct Packet {
  pub buf: Buffer,
  id:      i32,
}

impl Packet {
  pub fn new(id: i32) -> Packet {
    let mut buf = Buffer::new(vec![]);
    buf.write_varint(id);
    Packet { buf, id }
  }
  pub fn from_buf(data: Vec<u8>) -> Packet {
    let mut buf = Buffer::new(data);
    let id = buf.read_varint();
    Packet { buf, id }
  }
  pub fn id(&self) -> i32 {
    self.id
  }
  pub fn err(&self) -> &Option<BufferError> {
    self.buf.err()
  }
}
