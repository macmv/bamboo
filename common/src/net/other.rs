use crate::proto;
use prost::{DecodeError, EncodeError, Message};
use prost_types::Any;

#[derive(Debug)]
pub enum Other {
  Chunk(proto::Chunk),
  BossBar(proto::BossBar),
}

/// Creates a type url from the given keyword. This should be used to
/// endcode/decode Any types.
macro_rules! create_type_url {
  ($ty: expr) => {
    concat!("type.googleapis.com/google.rpc.", stringify!($ty))
  };
}

macro_rules! any_decode {
  [$any: expr, $($val: ident),*] => {
    match $any.type_url.as_str() {
      $(
        create_type_url!($val) => {
          match proto::$val::decode($any.value.as_slice()) {
            Ok(msg) => Ok(Other::$val(msg)),
            Err(e) => Err(e),
          }
        },
      )*
      _ => panic!("unknown type {}", $any.type_url),
    }
  }
}

macro_rules! any_encode {
  [$self: expr, $buf: expr, $($val: ident),*] => {
    match $self {
      $(
        Self::$val(pb) => match pb.encode($buf) {
          Ok(_) => Ok(create_type_url!($val)),
          Err(e) => Err(e),
        },
      )*
    }
  }
}

impl Other {
  pub fn from_any(pb: Any) -> Result<Self, DecodeError> {
    any_decode![pb, Chunk, BossBar]
  }
  pub fn to_any(&self) -> Result<Any, EncodeError> {
    let mut b = bytes::BytesMut::new();
    let name = any_encode![self, &mut b, Chunk, BossBar]?;

    dbg!(create_type_url!(Chunk));
    Ok(Any { type_url: name.to_string(), value: b.to_vec() })
  }
}
