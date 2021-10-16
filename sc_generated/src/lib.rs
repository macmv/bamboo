#![doc(
  html_playground_url = "https://play.rust-lang.org/",
  test(no_crate_inject, attr(deny(warnings)))
)]
#![feature(test)]

// use flexi_logger::{Duplicate, LogTarget, Logger};

mod chunk_pos;
pub mod net;
mod pos;
pub mod util;
pub mod version;

pub use chunk_pos::ChunkPos;
pub use pos::{Pos, PosError, PosIter};

// pub mod proto {
//   tonic::include_proto!("connection");
//
//   pub const FILE_DESCRIPTOR_SET: &[u8] =
// tonic::include_file_descriptor_set!("connection"); }
