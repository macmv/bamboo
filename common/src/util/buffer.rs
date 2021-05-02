use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{io, io::Cursor};

#[derive(Debug)]
pub struct Buffer {
  data: Cursor<Vec<u8>>,
  err: Option<io::Error>,
}

macro_rules! add_read {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self) -> $ty {
      if self.err.is_some() {
        return 0;
      }
      match self.data.$fn::<BigEndian>() {
        Ok(v) => v,
        Err(e) => {
          self.err = Some(e);
          0
        }
      }
    }
  };
}
// The same as add_read(), but with no type parameter
macro_rules! add_read_byte {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self) -> $ty {
      if self.err.is_some() {
        return 0;
      }
      match self.data.$fn() {
        Ok(v) => v,
        Err(e) => {
          self.err = Some(e);
          0
        }
      }
    }
  };
}

macro_rules! add_write {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self, v: $ty) {
      if self.err.is_some() {
        return;
      }
      match self.data.$fn::<BigEndian>(v) {
        Ok(()) => {}
        Err(e) => self.err = Some(e),
      }
    }
  };
}
// The same as add_read(), but with no type parameter
macro_rules! add_write_byte {
  ($fn: ident, $ty: ty) => {
    pub fn $fn(&mut self, v: $ty) {
      if self.err.is_some() {
        return;
      }
      match self.data.$fn(v) {
        Ok(()) => {}
        Err(e) => self.err = Some(e),
      }
    }
  };
}

impl Buffer {
  pub fn new(data: Vec<u8>) -> Self {
    Buffer { data: Cursor::new(data), err: None }
  }

  add_read_byte!(read_u8, u8);
  add_read!(read_u16, u16);
  add_read!(read_u32, u32);
  add_read!(read_u64, u64);
  add_read_byte!(read_i8, i8);
  add_read!(read_i16, i16);
  add_read!(read_i32, i32);
  add_read!(read_i64, i64);

  add_write_byte!(write_u8, u8);
  add_write!(write_u16, u16);
  add_write!(write_u32, u32);
  add_write!(write_u64, u64);
  add_write_byte!(write_i8, i8);
  add_write!(write_i16, i16);
  add_write!(write_i32, i32);
  add_write!(write_i64, i64);
}

#[cfg(test)]
mod tests {}
