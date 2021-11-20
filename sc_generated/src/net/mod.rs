// data::generate_protocol!();

pub mod cb {
  use super::*;

  include!(concat!(env!("OUT_DIR"), "/protocol/cb.rs"));
}
pub mod sb {
  use super::*;

  include!(concat!(env!("OUT_DIR"), "/protocol/sb.rs"));
}

mod other;
pub mod tcp;

pub fn clamp<T: PartialOrd + Copy>(mut a: T, min: T, max: T) -> T {
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
