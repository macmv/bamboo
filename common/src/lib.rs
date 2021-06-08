#![doc(
  html_playground_url = "https://play.rust-lang.org/",
  test(no_crate_inject, attr(deny(warnings)))
)]

// use flexi_logger::{Duplicate, LogTarget, Logger};
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::rolling_file::{
  policy::compound::{trigger::Trigger, CompoundPolicy},
  LogFile, RollingFileAppender,
};
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::time::{Duration, Instant};

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

#[derive(Debug)]
struct SizeOrTimeTrigger {
  size: u64,
  time: Duration,
  last: Instant,
}
struct Roller {}

impl SizeOrTimeTrigger {
  pub fn new(size: u64, time: Duration) -> Self {
    SizeOrTimeTrigger { size, time, last: Instant::now() }
  }
}

impl Trigger for SizeOrTimeTrigger {
  fn trigger(&self, file: &LogFile) -> anyhow::Result<bool> {
    if file.len_estimate() > self.size || self.last.elapsed() > self.time {
      self.last = Instant::now();
      Ok(true)
    } else {
      Ok(false)
    }
  }
}

impl Roller {}

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
      Box::new(CompoundPolicy::new(Box::new(Trigger::new()), Box::new(Roller::new()))),
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
