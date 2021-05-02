use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{error::Error, fmt, io, io::Cursor};

#[derive(Debug)]
pub struct BufferError {
  err: BufferErrorKind,
  pos: u64,
  reading: bool,
}

impl fmt::Display for BufferError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.reading {
      write!(f, "error while reading from buffer at index {}: {}", self.pos, self.err)
    } else {
      write!(f, "error while writing to buffer at index {}: {}", self.pos, self.err)
    }
  }
}

#[derive(Debug)]
pub enum BufferErrorKind {
  IO(io::Error),
  VarInt(),
}

impl fmt::Display for BufferErrorKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Self::IO(e) => write!(f, "{}", e),
      Self::VarInt() => write!(f, "varint is too long"),
    }
  }
}

impl Error for BufferError {}
impl Error for BufferErrorKind {}

#[derive(Debug)]
pub struct Buffer {
  data: Cursor<Vec<u8>>,
  err: Option<BufferError>,
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
          self.set_err(BufferErrorKind::IO(e), true);
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
          self.set_err(BufferErrorKind::IO(e), true);
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
        Err(e) => {
          self.set_err(BufferErrorKind::IO(e), false);
        }
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
        Err(e) => {
          self.set_err(BufferErrorKind::IO(e), false);
        }
      }
    }
  };
}

impl Buffer {
  pub fn new(data: Vec<u8>) -> Self {
    Buffer { data: Cursor::new(data), err: None }
  }

  pub fn err(&self) -> &Option<BufferError> {
    &self.err
  }

  pub fn set_err(&mut self, err: BufferErrorKind, reading: bool) {
    self.err = Some(BufferError { err, pos: self.data.position(), reading });
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

  pub fn read_varint(&mut self) -> i32 {
    let mut res: i32 = 0;
    for i in 0..5 {
      let read = self.read_u8();
      if i == 4 && read & 0b10000000 != 0 {
        // TODO: Custom error here
        self.set_err(BufferErrorKind::VarInt(), true);
        return 0;
      }

      let v = read & 0b01111111;
      res |= (v as i32) << (7 * i);

      if read & 0b10000000 == 0 {
        break;
      }
    }
    res
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn read_varint() {
    let mut buf = Buffer::new(vec![1]);
    assert_eq!(1, buf.read_varint());
    assert!(buf.err().is_none());

    let mut buf = Buffer::new(vec![127]);
    assert_eq!(127, buf.read_varint());
    assert!(buf.err().is_none());

    let mut buf = Buffer::new(vec![128, 2]);
    assert_eq!(256, buf.read_varint());
    assert!(buf.err().is_none());

    let mut buf = Buffer::new(vec![255, 255, 255, 255, 15]);
    assert_eq!(-1, buf.read_varint());
    assert!(buf.err().is_none());

    let mut buf = Buffer::new(vec![255, 255, 255, 255, 255]);
    assert_eq!(0, buf.read_varint());
    assert!(buf.err().is_some());
  }
}
