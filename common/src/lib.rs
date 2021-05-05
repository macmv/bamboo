#![doc(
  html_playground_url = "https://play.rust-lang.org/",
  test(no_crate_inject, attr(deny(warnings)))
)]

use flexi_logger::{Duplicate, LogTarget, Logger};

pub mod math;
pub mod net;
pub mod registry;
pub mod util;
pub mod version;

pub use registry::Registry;

pub mod proto {
  tonic::include_proto!("connection");

  pub const FILE_DESCRIPTOR_SET: &'static [u8] = tonic::include_file_descriptor_set!("connection");
}

/// Initializes logger. Might do more things in the future.
pub fn init() {
  Logger::with_env_or_str("info")
    .log_target(LogTarget::File)
    .directory("log")
    .duplicate_to_stdout(Duplicate::All)
    .format(flexi_logger::opt_format)
    .start()
    .unwrap();
}
