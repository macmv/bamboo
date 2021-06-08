#![doc(
  html_playground_url = "https://play.rust-lang.org/",
  test(no_crate_inject, attr(deny(warnings)))
)]

// use flexi_logger::{Duplicate, LogTarget, Logger};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::{
  policy::compound::{roll::Roll, trigger::size::SizeTrigger, CompoundPolicy},
  RollingFileAppender,
};
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::{
  fs,
  path::Path,
  time::{Duration, Instant},
};

pub mod math;
pub mod net;
pub mod registry;
pub mod util;
pub mod version;

pub use registry::Registry;

pub mod proto {
  tonic::include_proto!("connection");

  pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("connection");
}

/// Deletes old files older than age any time they are found in the log
/// directory.
#[derive(Debug)]
struct DeleteWindowRoller {
  age: Duration,
}

impl Roll for DeleteWindowRoller {
  fn roll(&self, file: &Path) -> anyhow::Result<()> {
    fs::remove_file(file).map_err(Into::into)
  }
}

impl DeleteWindowRoller {
  /// Returns a new `DeleteRoller`.
  pub fn new(age: Duration) -> Self {
    DeleteWindowRoller { age }
  }
}

/// Initializes logger. Might do more things in the future.
pub fn init(name: &str) {
  // Put line numbers in debug builds, but not in release builds.
  #[cfg(debug_assertions)]
  let pat = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S:%f)} {f}:{L} [{h({l})}] {m}{n}");
  #[cfg(not(debug_assertions))]
  let pat = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S:%f)} [{h({l})}] {m}{n}");

  let stdout = ConsoleAppender::builder().encoder(Box::new(pat.clone())).build();
  let disk = RollingFileAppender::builder()
    .encoder(Box::new(pat))
    .build(
      format!("log/{}.log", name),
      Box::new(CompoundPolicy::new(
        Box::new(SizeTrigger::new(100 * 1024)),
        Box::new(DeleteWindowRoller::new(Duration::from_secs(60 * 60 * 24 * 7))),
      )),
    )
    .unwrap();

  let config = Config::builder()
    .appender(Appender::builder().build("stdout", Box::new(stdout)))
    .appender(Appender::builder().build("disk", Box::new(disk)))
    .build(Root::builder().appender("stdout").appender("disk").build(LevelFilter::Info))
    .unwrap();

  log4rs::init_config(config).unwrap();

  // Logger::with_env_or_str("info")
  //   .log_target(LogTarget::File)
  //   .directory("log")
  //   .duplicate_to_stdout(Duplicate::All)
  //   .format(flexi_logger::opt_format)
  //   .start()
  //   .unwrap();
}
