use super::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};

macro_rules! num_impl {
  ($ty:ty, $read:ident, $write:ident) => {
    impl MessageRead for $ty {
      fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
        m.$read()
      }
    }
    impl MessageWrite for $ty {
      fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
        m.$write(*self)
      }
    }
  };
}

num_impl!(bool, read_bool, write_bool);
num_impl!(u8, read_u8, write_u8);
num_impl!(i8, read_i8, write_i8);
num_impl!(u16, read_u16, write_u16);
num_impl!(i16, read_i16, write_i16);
num_impl!(u32, read_u32, write_u32);
num_impl!(i32, read_i32, write_i32);
num_impl!(u64, read_u64, write_u64);
num_impl!(i64, read_i64, write_i64);
num_impl!(f32, read_f32, write_f32);
num_impl!(f64, read_f64, write_f64);

impl<T> MessageRead for Option<T>
where
  T: MessageRead,
{
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(if m.read()? { Some(m.read()?) } else { None })
  }
}
impl<T> MessageWrite for Option<T>
where
  T: MessageWrite,
{
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write(self.is_some())?;
    match self {
      Some(v) => v.write(m),
      None => m.write(false),
    }
  }
}
