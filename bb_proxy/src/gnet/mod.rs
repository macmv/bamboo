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

pub fn clamp<T: PartialOrd + Copy, N: Into<T>>(a: T, min: N, max: N) -> T {
    let min = min.into();
    let max = max.into();
    if a < min {
        return min
    }
    if a > max {
        return max
    }
    a
}

pub enum Hand {
  Main,
  Off,
}

// pub use other::Other;
