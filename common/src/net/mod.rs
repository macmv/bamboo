// data::generate_protocol!();

pub mod cb {
  include!(concat!(env!("OUT_DIR"), "/protocol/cb.rs"));
}
pub mod sb {
  include!(concat!(env!("OUT_DIR"), "/protocol/sb.rs"));
}

mod other;
pub mod tcp;

pub use other::Other;
