// data::generate_protocol!();

pub mod cb {
  use super::clamp;

  include!(concat!(env!("OUT_DIR"), "/protocol/cb.rs"));
}
pub mod sb {
  use super::clamp;

  include!(concat!(env!("OUT_DIR"), "/protocol/sb.rs"));
}

mod other;
pub mod tcp;

pub fn clamp<T: Ord + Copy>(mut a: T, min: T, max: T) -> T {
  if a < min {
    a = min
  }
  if a > max {
    a = max
  }
  a
}

// pub use other::Other;
