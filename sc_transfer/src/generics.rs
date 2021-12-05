use super::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};
use std::{
  collections::{HashMap, HashSet},
  hash::Hash,
};

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

impl MessageRead for String {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    m.read_str()
  }
}
impl MessageWrite for String {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_str(self)
  }
}

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
    m.write(&self.is_some())?;
    match self {
      Some(v) => v.write(m),
      None => Ok(()),
    }
  }
}
impl<T> MessageRead for Vec<T>
where
  T: MessageRead,
{
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    let len: usize = m.read_u32()?.try_into().unwrap();
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
      out.push(m.read()?);
    }
    Ok(out)
  }
}
impl<T> MessageWrite for Vec<T>
where
  T: MessageWrite,
{
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_u32(self.len().try_into().unwrap())?;
    for v in self {
      v.write(m)?;
    }
    Ok(())
  }
}
impl<K, V> MessageRead for HashMap<K, V>
where
  K: MessageRead + Eq + Hash,
  V: MessageRead,
{
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    let len: usize = m.read_u32()?.try_into().unwrap();
    let mut out = HashMap::with_capacity(len);
    for _ in 0..len {
      out.insert(m.read()?, m.read()?);
    }
    Ok(out)
  }
}
impl<K, V> MessageWrite for HashMap<K, V>
where
  K: MessageWrite,
  V: MessageWrite,
{
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_u32(self.len().try_into().unwrap())?;
    for (k, v) in self {
      k.write(m)?;
      v.write(m)?;
    }
    Ok(())
  }
}
impl<T> MessageRead for HashSet<T>
where
  T: MessageRead + Eq + Hash,
{
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    let len: usize = m.read_u32()?.try_into().unwrap();
    let mut out = HashSet::with_capacity(len);
    for _ in 0..len {
      out.insert(m.read()?);
    }
    Ok(out)
  }
}
impl<T> MessageWrite for HashSet<T>
where
  T: MessageWrite,
{
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write_u32(self.len().try_into().unwrap())?;
    for v in self {
      v.write(m)?;
    }
    Ok(())
  }
}

// I cannot figure out how to call `m.read()?` multiple times with const
// generics. So, MessageRead only works for arrays up to 32 elements.
// MessageWrite works for any length array.

macro_rules! array_impl {
  { $n:expr, $t:ident $($ts:ident)* } => {
    impl<T: MessageRead> MessageRead for [T; $n] {
      fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
        Ok([$t::read(m)?, $($ts::read(m)?),*])
      }
    }
    array_impl! { ($n - 1), $($ts)* }
  };
  { $n:expr, } => {
    impl<T> MessageRead for [T; $n] {
      fn read(_m: &mut MessageReader) -> Result<Self, ReadError> { Ok([]) }
    }
  };
}

array_impl! { 32, T T T T T T T T T T T T T T T T T T T T T T T T T T T T T T T T }

impl<T, const N: usize> MessageWrite for [T; N]
where
  T: MessageWrite,
{
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    for v in self {
      v.write(m)?;
    }
    Ok(())
  }
}

macro_rules! tuple_impls {
    ($(
      $Tuple:ident {
        $(($idx:tt) -> $T:ident)+
      }
    )+) => {
    $(
      impl<$($T: MessageRead),+> MessageRead for ($($T,)+) {
        fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
          Ok(($($T::read(m)?,)+))
        }
      }
      impl<$($T: MessageWrite),+> MessageWrite for ($($T,)+) {
        fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
          $(
            self.$idx.write(m)?;
          )+
          Ok(())
        }
      }
    )+
  };
}

tuple_impls! {
  Tuple1 {
    (0) -> A
  }
  Tuple2 {
    (0) -> A
    (1) -> B
  }
  Tuple3 {
    (0) -> A
    (1) -> B
    (2) -> C
  }
  Tuple4 {
    (0) -> A
    (1) -> B
    (2) -> C
    (3) -> D
  }
}
