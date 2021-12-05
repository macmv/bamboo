/// A trait to deserialize data from a buffer. This is used in the protocol, to
/// simplify code generation.
pub trait ReadSc {
  fn read_sc(buf: &mut tcp::Packet) -> Self;
}
/// A trait to serialize data to a buffer. This is used in the protocol, to
/// simplify code generation.
pub trait WriteSc {
  fn write_sc(&self, buf: &mut tcp::Packet);
}

impl ReadSc for i32 {
  fn read_sc(buf: &mut tcp::Packet) -> i32 {
    buf.read_i32()
  }
}
impl WriteSc for i32 {
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_i32(self)
  }
}
