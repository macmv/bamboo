use common::{
  math::Pos,
  util::{Buffer, BufferError},
  version::ProtocolVersion,
};

#[derive(Debug)]
pub struct Packet {
  buf: Buffer,
  id:  i32,
  ver: ProtocolVersion,
}

macro_rules! add_writer {
  ($name: ident, $arg: ty) => {
    pub fn $name(&mut self, v: $arg) {
      self.buf.$name(v)
    }
  };
}
macro_rules! add_reader {
  ($name: ident, $ret: ty) => {
    pub fn $name(&mut self) -> $ret {
      self.buf.$name()
    }
  };
}

impl Packet {
  pub fn new(id: i32, ver: ProtocolVersion) -> Packet {
    let mut buf = Buffer::new(vec![]);
    buf.write_varint(id);
    Packet { buf, id, ver }
  }
  pub fn from_buf(data: Vec<u8>, ver: ProtocolVersion) -> Packet {
    let mut buf = Buffer::new(data);
    let id = buf.read_varint();
    Packet { buf, id, ver }
  }
  pub fn id(&self) -> i32 {
    self.id
  }
  pub fn err(&self) -> &Option<BufferError> {
    self.buf.err()
  }
  pub fn serialize(self) -> Vec<u8> {
    self.buf.into_inner()
  }

  add_writer!(write_u8, u8);
  add_writer!(write_u16, u16);
  add_writer!(write_u32, u32);
  add_writer!(write_u64, u64);
  add_writer!(write_i8, i8);
  add_writer!(write_i16, i16);
  add_writer!(write_i32, i32);
  add_writer!(write_i64, i64);

  add_writer!(write_varint, i32);
  add_writer!(write_str, &str);
  add_writer!(write_bool, bool);

  add_reader!(read_u8, u8);
  add_reader!(read_u16, u16);
  add_reader!(read_u32, u32);
  add_reader!(read_u64, u64);
  add_reader!(read_i8, i8);
  add_reader!(read_i16, i16);
  add_reader!(read_i32, i32);
  add_reader!(read_i64, i64);

  add_reader!(read_varint, i32);
  add_reader!(read_str, String);
  add_reader!(read_bool, bool);

  /// This parses a postition from a grpc packet (always new format), and then
  /// writes the long back into the buffer, with either the new or old format.
  /// The new format will be used for any packet with version 1.14 or later.
  pub fn write_pos(&mut self, v: u64) {
    let pos = Pos::from_u64(v);
    if self.ver < ProtocolVersion::V1_14 {
      self.buf.write_u64(pos.to_old_u64());
    } else {
      self.buf.write_u64(pos.to_u64());
    }
  }
}
