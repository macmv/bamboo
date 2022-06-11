// data::generate_protocol!();

#[cfg(not(doctest))]
pub mod cb {
  use super::*;

  include!(concat!(env!("OUT_DIR"), "/protocol/cb.rs"));
}
#[cfg(not(doctest))]
pub mod sb {
  use super::*;

  include!(concat!(env!("OUT_DIR"), "/protocol/sb.rs"));
}

#[cfg(doctest)]
pub mod cb {
  pub enum Packet {}
}
#[cfg(doctest)]
pub mod sb {
  pub enum Packet {}
}

mod other;
pub mod tcp;

pub fn clamp<T: PartialOrd + Copy, N: Into<T>>(mut a: T, min: N, max: N) -> T {
  let min = min.into();
  let max = max.into();
  if a < min {
    a = min
  }
  if a > max {
    a = max
  }
  a
}

pub enum Hand {
  Main,
  Off,
}

// pub use other::Other;
