use crate::{
  util::{nbt::NBT, Buffer, BufferError, Item, UUID},
  version::ProtocolVersion,
  Pos,
};
use std::{
  collections::{HashMap, HashSet},
  convert::TryInto,
  hash::Hash,
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
    self.buf.to_vec()
  }

  add_writer!(write_u8, u8);
  add_writer!(write_u16, u16);
  add_writer!(write_u32, u32);
  add_writer!(write_u64, u64);
  add_writer!(write_i8, i8);
  add_writer!(write_i16, i16);
  add_writer!(write_i32, i32);
  add_writer!(write_i64, i64);

  add_writer!(write_buf, &[u8]);

  add_writer!(write_f32, f32);
  add_writer!(write_f64, f64);
  add_writer!(write_varint, i32);
  add_writer!(write_str, &str);
  add_writer!(write_bool, bool);
  add_writer!(write_fixed_int, f64);

  add_reader!(read_u8, u8);
  add_reader!(read_u16, u16);
  add_reader!(read_u32, u32);
  add_reader!(read_u64, u64);
  add_reader!(read_i8, i8);
  add_reader!(read_i16, i16);
  add_reader!(read_i32, i32);
  add_reader!(read_i64, i64);

  add_reader!(read_f32, f32);
  add_reader!(read_f64, f64);
  add_reader!(read_varint, i32);
  add_reader!(read_bool, bool);
  add_reader!(read_all, Vec<u8>);

  pub fn read_str(&mut self, max_len: u64) -> String {
    self.buf.read_str(max_len)
  }
  pub fn read_buf(&mut self, len: usize) -> Vec<u8> {
    self.buf.read_buf(len)
  }

  pub fn remaining(&self) -> usize {
    self.buf.len() - self.buf.index()
  }
  pub fn len(&self) -> usize {
    self.buf.len()
  }
  pub fn is_empty(&self) -> bool {
    self.buf.len() == 0
  }

  /// This writes the given block position as a long into the buffer. The old
  /// format is used on 1.8 - 1.13, and the new format will be used for any
  /// packet with version 1.14 or later.
  pub fn write_pos(&mut self, p: Pos) {
    if self.ver < ProtocolVersion::V1_14 {
      self.buf.write_u64(p.to_old_u64());
    } else {
      self.buf.write_u64(p.to_u64());
    }
  }

  /// This parses a postition from the internal buffer (format depends on the
  /// version), and then returns that as a Pos struct.
  pub fn read_pos(&mut self) -> Pos {
    let num = self.read_u64();
    if self.ver < ProtocolVersion::V1_14 {
      Pos::from_old_u64(num)
    } else {
      Pos::from_u64(num)
    }
  }

  /// Reads an nbt tag from self.
  pub fn read_nbt(&mut self) -> NBT {
    NBT::deserialize_buf(&mut self.buf).unwrap()
  }

  /// Reads a length prefixed array of integers.
  pub fn read_i32_arr(&mut self) -> Vec<i32> {
    let len = self.read_varint().try_into().unwrap();
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
      out.push(self.read_i32());
    }
    out
  }

  /// This parses an item from the internal buffer (format depends on the
  /// version).
  pub fn read_item(&mut self) -> Item {
    if self.ver < ProtocolVersion::V1_13 {
      let id = self.read_i16();
      let mut count = 0;
      let mut damage = 0;
      let mut nbt = NBT::empty("");
      if id != -1 {
        count = self.read_u8();
        damage = self.read_i16();
        nbt = self.read_nbt();
      }
      Item::new(id.into(), count, damage, nbt)
    } else {
      unreachable!("invalid version: {:?}", self.ver);
    }
  }

  /// This writes the given item to the internal buffer (format depends on the
  /// version).
  pub fn write_item(&mut self, item: &Item) {
    if self.ver < ProtocolVersion::V1_13 {
      self.write_i16(item.id() as i16);
      if item.id() != -1 {
        self.write_u8(item.count());
        self.write_i16(0); // Item damage
        self.write_u8(0); // TODO: Write nbt data
      }
    } else {
      unreachable!("invalid version: {:?}", self.ver);
    }
  }

  /// Reads 16 bytes from the buffer, and returns that as a big endian UUID.
  pub fn read_uuid(&mut self) -> UUID {
    UUID::from_bytes(self.read_buf(16).try_into().unwrap())
  }

  /// This writes a UUID into the buffer (in big endian format).
  pub fn write_uuid(&mut self, v: UUID) {
    self.write_buf(&v.as_be_bytes());
  }

  /// Reads a block hit result. This (for whatever dumb reason) is part of the
  /// packet buffer in 1.17, and is literally called ONCE. So, because reasons,
  /// I need to implement it as well.
  pub fn read_block_hit(&mut self) -> ((f32, f32, f32), i32, Pos, bool) {
    let pos = self.read_pos();
    let dir = self.read_varint();
    let x = self.read_f32();
    let y = self.read_f32();
    let z = self.read_f32();
    let hit = self.read_bool();
    return ((pos.x() as f32 + x, pos.y() as f32 + y, pos.z() as f32 + z), dir, pos, hit);
  }

  /// Reads a list from the packet. This is new to 1.17, and simplifies a bunch
  /// of small for loops in previous versions.
  pub fn read_list<T>(&mut self, val: impl Fn(&mut Packet) -> T) -> Vec<T> {
    let len = self.read_varint().try_into().unwrap();
    let list = Vec::with_capacity(len);
    for i in 0..len {
      list.push(val(self));
    }
    list
  }
  /// Reads a list from the packet. If the length is greater than `max`, this
  /// fails. This is new to 1.17, and simplifies a bunch of small for loops in
  /// previous versions.
  pub fn read_list_max<T>(&mut self, val: impl Fn(&mut Packet) -> T, max: usize) -> Vec<T> {
    let len = self.read_varint().try_into().unwrap();
    if len > max {
      panic!("length {} greater than max {}", len, max);
    }
    let list = Vec::with_capacity(len);
    for i in 0..len {
      list.push(val(self));
    }
    list
  }

  /// Reads a HashMap from the packet. This is new to 1.17, and simplifies a
  /// bunch of small for loops in previous versions.
  pub fn read_map<K: Eq + Hash, V>(
    &mut self,
    key: impl Fn(&mut Packet) -> K,
    val: impl Fn(&mut Packet) -> V,
  ) -> HashMap<K, V> {
    let len = self.read_varint().try_into().unwrap();
    let map = HashMap::with_capacity(len);
    for i in 0..len {
      map.insert(key(self), val(self));
    }
    map
  }

  /// Reads a HashSet from the packet. This is new to 1.17, and simplifies a
  /// bunch of small for loops in previous versions.
  pub fn read_set<T: Eq + Hash>(&mut self, val: impl Fn(&mut Packet) -> T) -> HashSet<T> {
    let len = self.read_varint().try_into().unwrap();
    let set = HashSet::with_capacity(len);
    for i in 0..len {
      set.insert(val(self));
    }
    set
  }
  /// Reads a HashSet from the packet. If the length is greater than `max`, this
  /// fails. This is new to 1.17, and simplifies a bunch of small for loops in
  /// previous versions.
  pub fn read_set_max<T: Eq + Hash>(
    &mut self,
    val: impl Fn(&mut Packet) -> T,
    max: usize,
  ) -> HashSet<T> {
    let len = self.read_varint().try_into().unwrap();
    if len > max {
      panic!("length {} greater than max {}", len, max);
    }
    let set = HashSet::with_capacity(len);
    for i in 0..len {
      set.insert(val(self));
    }
    set
  }

  /// Reads a boolean. If true, the closure is called, and the returned value is
  /// wrapped in Some. Otherwise, this returns None.
  pub fn read_option<T>(&mut self, val: impl FnOnce(&mut Packet) -> T) -> Option<T> {
    if self.read_bool() {
      Some(val(self))
    } else {
      None
    }
  }

  pub fn read_varint_arr(&mut self) -> Vec<i32> {
    self.read_list(Self::read_varint)
  }
}
