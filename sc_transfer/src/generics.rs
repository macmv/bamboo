use super::{MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, WriteError};
use std::{
  collections::{HashMap, HashSet},
  hash::Hash,
  marker::PhantomData,
  net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

macro_rules! num_impl {
  ($ty:ty, $read:ident, $write:ident) => {
    impl MessageRead<'_> for $ty {
      fn read(m: &mut MessageReader) -> Result<Self, ReadError> { m.$read() }
    }
    impl MessageWrite for $ty {
      fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> { m.$write(*self) }
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

impl<'a> MessageRead<'a> for &'a str {
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> { m.read_str() }
}
impl MessageWrite for &str {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> { m.write_str(self) }
}

impl MessageRead<'_> for String {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { Ok(m.read_str()?.into()) }
}
impl MessageWrite for String {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> { m.write_str(self) }
}

impl<'a, T> MessageRead<'a> for Option<T>
where
  T: MessageRead<'a>,
{
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
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
impl<'a, T> MessageRead<'a> for Vec<T>
where
  T: MessageRead<'a>,
{
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
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
impl<'a, K, V> MessageRead<'a> for HashMap<K, V>
where
  K: MessageRead<'a> + Eq + Hash,
  V: MessageRead<'a>,
{
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
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
impl<'a, T> MessageRead<'a> for HashSet<T>
where
  T: MessageRead<'a> + Eq + Hash,
{
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
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
    impl<'a, T: MessageRead<'a>> MessageRead<'a> for [T; $n] {
      fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
        Ok([$t::read(m)?, $($ts::read(m)?),*])
      }
    }
    array_impl! { ($n - 1), $($ts)* }
  };
  { $n:expr, } => {
    impl<T> MessageRead<'_> for [T; $n] {
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
      impl<'a, $($T: MessageRead<'a>),+> MessageRead<'a> for ($($T,)+) {
        fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
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

impl<T> MessageRead<'_> for PhantomData<T> {
  fn read(_: &mut MessageReader) -> Result<Self, ReadError> { Ok(PhantomData::default()) }
}
impl<T> MessageWrite for PhantomData<T> {
  fn write(&self, _: &mut MessageWriter) -> Result<(), WriteError> { Ok(()) }
}

impl MessageRead<'_> for SocketAddr {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(match m.read_u8()? {
      0 => SocketAddr::V4(m.read()?),
      1 => SocketAddr::V6(m.read()?),
      v => panic!("unknown socket addr type {}", v),
    })
  }
}
impl MessageWrite for SocketAddr {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    match self {
      SocketAddr::V4(addr) => {
        m.write_u8(0)?;
        m.write(addr)?;
      }
      SocketAddr::V6(addr) => {
        m.write_u8(1)?;
        m.write(addr)?;
      }
    }
    Ok(())
  }
}

impl MessageRead<'_> for SocketAddrV4 {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(SocketAddrV4::new(m.read()?, m.read()?))
  }
}
impl MessageWrite for SocketAddrV4 {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write(self.ip())?;
    m.write(&self.port())?;
    Ok(())
  }
}
impl MessageRead<'_> for SocketAddrV6 {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    Ok(SocketAddrV6::new(m.read()?, m.read()?, m.read()?, m.read()?))
  }
}
impl MessageWrite for SocketAddrV6 {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    m.write(self.ip())?;
    m.write(&self.port())?;
    m.write(&self.flowinfo())?;
    m.write(&self.scope_id())?;
    Ok(())
  }
}
impl MessageRead<'_> for Ipv4Addr {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    // 4 8-bit numbers
    Ok(Ipv4Addr::new(m.read()?, m.read()?, m.read()?, m.read()?))
  }
}
impl MessageWrite for Ipv4Addr {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    // 4 8-bit numbers
    for oct in self.octets() {
      m.write(&oct)?;
    }
    Ok(())
  }
}
impl MessageRead<'_> for Ipv6Addr {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    // 8 16-bit numbers
    Ok(Ipv6Addr::new(
      m.read()?,
      m.read()?,
      m.read()?,
      m.read()?,
      m.read()?,
      m.read()?,
      m.read()?,
      m.read()?,
    ))
  }
}
impl MessageWrite for Ipv6Addr {
  fn write(&self, m: &mut MessageWriter) -> Result<(), WriteError> {
    // 8 16-bit numbers
    for seg in self.segments() {
      m.write(&seg)?;
    }
    Ok(())
  }
}
