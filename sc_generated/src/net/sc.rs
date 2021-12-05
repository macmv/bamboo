use super::tcp;

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

macro_rules! sc_simple {
  ($ty:ty, $read:ident, $write:ident) => {
    impl ReadSc for $ty {
      fn read_sc(buf: &mut tcp::Packet) -> Self {
        buf.$read()
      }
    }
    impl WriteSc for $ty {
      fn write_sc(&self, buf: &mut tcp::Packet) {
        buf.$write(*self)
      }
    }
  };
}

sc_simple!(bool, read_bool, write_bool);
sc_simple!(u8, read_u8, write_u8);
sc_simple!(i8, read_i8, write_i8);
sc_simple!(u16, read_u16, write_u16);
sc_simple!(i16, read_i16, write_i16);
sc_simple!(u32, read_u32, write_u32);
sc_simple!(i32, read_i32, write_i32);
sc_simple!(u64, read_u64, write_u64);
sc_simple!(i64, read_i64, write_i64);
sc_simple!(f32, read_f32, write_f32);
sc_simple!(f64, read_f64, write_f64);

impl<T> ReadSc for Option<T>
where
  T: ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_option(|buf| T::read_sc(buf))
  }
}
impl<T> WriteSc for Option<T>
where
  T: WriteSc,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_option(self, |buf, v| v.write_sc(buf))
  }
}

impl<T> ReadSc for Vec<T>
where
  T: ReadSc,
{
  fn read_sc(buf: &mut tcp::Packet) -> Self {
    buf.read_list(|buf| T::read_sc(buf))
  }
}
impl<T> WriteSc for Vec<T>
where
  T: WriteSc,
{
  fn write_sc(&self, buf: &mut tcp::Packet) {
    buf.write_list(self, |buf, v| v.write_sc(buf))
  }
}
