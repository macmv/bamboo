use super::{
  EnumRead, EnumReader, MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError,
  StructRead, StructReader, WriteError,
};
use std::{
  collections::{HashMap, HashSet},
  hash::{BuildHasher, Hash},
  io::Write,
  marker::PhantomData,
  net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

impl<T> MessageWrite for &T
where
  T: ?Sized + MessageWrite,
{
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write::<T>(self)
  }
}

macro_rules! num_impl {
  ($ty:ty, $read:ident, $write:ident) => {
    impl MessageRead<'_> for $ty {
      fn read(m: &mut MessageReader) -> Result<Self, ReadError> { m.$read() }
    }
    impl MessageWrite for $ty {
      fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
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

macro_rules! num_nonzero_impl {
  ($ty:ty, $read:ident, $write:ident) => {
    impl MessageRead<'_> for $ty {
      fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
        m.$read().map(|num| <$ty>::new(num))?.ok_or(crate::ValidReadError::InvalidNonZero.into())
      }
    }
    impl MessageWrite for $ty {
      fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
        m.$write(self.get())
      }
    }
  };
}

num_nonzero_impl!(std::num::NonZeroU8, read_u8, write_u8);
num_nonzero_impl!(std::num::NonZeroI8, read_i8, write_i8);
num_nonzero_impl!(std::num::NonZeroU16, read_u16, write_u16);
num_nonzero_impl!(std::num::NonZeroI16, read_i16, write_i16);
num_nonzero_impl!(std::num::NonZeroU32, read_u32, write_u32);
num_nonzero_impl!(std::num::NonZeroI32, read_i32, write_i32);
num_nonzero_impl!(std::num::NonZeroU64, read_u64, write_u64);
num_nonzero_impl!(std::num::NonZeroI64, read_i64, write_i64);

impl<'a> MessageRead<'a> for &'a str {
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> { m.read_str() }
}
impl MessageWrite for &str {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_str(self)
  }
}

impl MessageRead<'_> for String {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { Ok(m.read_str()?.into()) }
}
impl MessageWrite for String {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_str(self)
  }
}

impl<'a, T> MessageRead<'a> for Option<T>
where
  T: MessageRead<'a>,
{
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> { m.read_enum() }
}
impl<'a, T> EnumRead<'a> for Option<T>
where
  T: MessageRead<'a>,
{
  fn read_enum(mut m: EnumReader<'a>) -> Result<Self, ReadError> {
    match m.variant() {
      0 => Ok(None),
      1 => Ok(Some(m.must_read(0)?)),
      _ => Err(m.invalid_variant()),
    }
  }
}
impl<T> MessageWrite for Option<T>
where
  T: MessageWrite,
{
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_enum(if self.is_some() { 1 } else { 0 }, if self.is_some() { 1 } else { 0 }, |m| {
      if let Some(v) = self {
        m.write(v)
      } else {
        Ok(())
      }
    })
  }
}
impl<'a> MessageRead<'a> for &'a [u8] {
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> { m.read_bytes() }
}
impl MessageWrite for &[u8] {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_bytes(self)
  }
}
impl<'a, T> MessageRead<'a> for Vec<T>
where
  T: MessageRead<'a>,
{
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> { m.read_list::<T>()?.collect() }
}
impl<T> MessageWrite for Vec<T>
where
  T: MessageWrite,
{
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_list(self.iter())
  }
}

impl<'a, K, V, B> MessageRead<'a> for HashMap<K, V, B>
where
  K: MessageRead<'a> + Eq + Hash,
  V: MessageRead<'a>,
  B: BuildHasher + Default,
{
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
    m.read_list::<(K, V)>()?.collect()
  }
}
impl<K, V, B> MessageWrite for HashMap<K, V, B>
where
  K: MessageWrite,
  V: MessageWrite,
  B: BuildHasher,
{
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_list(self.iter())
  }
}
impl<'a, T> MessageRead<'a> for HashSet<T>
where
  T: MessageRead<'a> + Eq + Hash,
{
  fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> { m.read_list::<T>()?.collect() }
}
impl<T> MessageWrite for HashSet<T>
where
  T: MessageWrite,
{
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_list(self.iter())
  }
}

// I cannot figure out how to call `m.read()?` multiple times with const
// generics. So, MessageRead only works for arrays up to 32 elements.
// MessageWrite works for any length array.

macro_rules! ignore {
  ( $ign:expr, $val:expr ) => {
    $val
  };
}
macro_rules! count {
  ( $($v:expr)+ ) => {
    0 $( + ignore!($v, 1) )+
  }
}

macro_rules! array_impl {
  { $n:expr, $t:ident $($ts:ident)* } => {
    impl<'a, T> MessageRead<'a> for [T; $n]
      where T: MessageRead<'a>,
    {
      fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
        let mut iter = m.read_list::<T>()?;
        assert_eq!(iter.len(), count!($t $($ts)*));
        Ok([iter.next().unwrap()?, $(ignore!($ts, iter.next().unwrap()?)),*])
      }
    }
    array_impl! { ($n - 1), $($ts)* }
  };
  { $n:expr, } => {
    impl<'a, T> MessageRead<'a> for [T; $n]
      where T: MessageRead<'a>,
    {
      fn read(m: &mut MessageReader<'a>) -> Result<Self, ReadError> {
        let iter = m.read_list::<T>()?;
        assert_eq!(iter.len(), 0);
        Ok([])
      }
    }
  };
}

array_impl! { 32, T T T T T T T T T T T T T T T T T T T T T T T T T T T T T T T T }

impl<T, const N: usize> MessageWrite for [T; N]
where
  T: MessageWrite,
{
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_list(self.iter())
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
          m.read_struct()
        }
      }

      impl<'a, $($T: MessageRead<'a>),+> StructRead<'a> for ($($T,)+) {
        fn read_struct(mut m: StructReader<'a>) -> Result<Self, ReadError> {
          Ok(($(m.must_read::<$T>($idx)?,)+))
        }
      }
      impl<$($T: MessageWrite),+> MessageWrite for ($($T,)+) {
        fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
          m.write_struct(count!($($T)+), |m| {
            $(
              self.$idx.write(m)?;
            )+
            Ok(())
          })
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
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { m.read_struct() }
}
impl<T> StructRead<'_> for PhantomData<T> {
  fn read_struct(_: StructReader) -> Result<Self, ReadError> { Ok(PhantomData) }
}
impl<T> MessageWrite for PhantomData<T> {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_struct(0, |_| Ok(()))
  }
}

impl MessageRead<'_> for SocketAddr {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { m.read_enum() }
}

impl EnumRead<'_> for SocketAddr {
  fn read_enum(mut m: EnumReader) -> Result<Self, ReadError> {
    match m.variant() {
      0 => Ok(SocketAddr::V4(m.must_read(0)?)),
      1 => Ok(SocketAddr::V6(m.must_read(0)?)),
      _ => Err(m.invalid_variant()),
    }
  }
}
impl MessageWrite for SocketAddr {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_enum(
      match self {
        SocketAddr::V4(_) => 0,
        SocketAddr::V6(_) => 1,
      },
      1,
      |m| match self {
        SocketAddr::V4(addr) => m.write(addr),
        SocketAddr::V6(addr) => m.write(addr),
      },
    )
  }
}

impl MessageRead<'_> for SocketAddrV4 {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { m.read_struct() }
}
impl StructRead<'_> for SocketAddrV4 {
  fn read_struct(mut m: StructReader) -> Result<Self, ReadError> {
    Ok(SocketAddrV4::new(m.must_read(0)?, m.must_read(1)?))
  }
}
impl MessageWrite for SocketAddrV4 {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_struct(2, |m| {
      m.write(self.ip())?;
      m.write(&self.port())
    })
  }
}
impl MessageRead<'_> for SocketAddrV6 {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> { m.read_struct() }
}
impl StructRead<'_> for SocketAddrV6 {
  fn read_struct(mut m: StructReader) -> Result<Self, ReadError> {
    Ok(SocketAddrV6::new(m.must_read(0)?, m.must_read(1)?, m.must_read(2)?, m.must_read(3)?))
  }
}
impl MessageWrite for SocketAddrV6 {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    m.write_struct(4, |m| {
      m.write(self.ip())?;
      m.write(&self.port())?;
      m.write(&self.flowinfo())?;
      m.write(&self.scope_id())
    })
  }
}
impl MessageRead<'_> for Ipv4Addr {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    // 4 8-bit numbers
    let mut iter = m.read_list()?;
    Ok(Ipv4Addr::new(
      iter.next().unwrap()?,
      iter.next().unwrap()?,
      iter.next().unwrap()?,
      iter.next().unwrap()?,
    ))
  }
}
impl MessageWrite for Ipv4Addr {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    // 4 8-bit numbers
    m.write_list(self.octets().iter())
  }
}
impl MessageRead<'_> for Ipv6Addr {
  fn read(m: &mut MessageReader) -> Result<Self, ReadError> {
    // 8 16-bit numbers
    let mut iter = m.read_list()?;
    Ok(Ipv6Addr::new(
      iter.next().unwrap()?,
      iter.next().unwrap()?,
      iter.next().unwrap()?,
      iter.next().unwrap()?,
      iter.next().unwrap()?,
      iter.next().unwrap()?,
      iter.next().unwrap()?,
      iter.next().unwrap()?,
    ))
  }
}
impl MessageWrite for Ipv6Addr {
  fn write<W: Write>(&self, m: &mut MessageWriter<W>) -> Result<(), WriteError> {
    // 8 16-bit numbers
    m.write_list(self.segments().iter())
  }
}
