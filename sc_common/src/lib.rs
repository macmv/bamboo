#![doc(
  html_playground_url = "https://play.rust-lang.org/",
  test(no_crate_inject, attr(deny(warnings)))
)]
#![feature(test)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate async_trait;

// use flexi_logger::{Duplicate, LogTarget, Logger};
use log::LevelFilter;
use log4rs::{
  append::{console::ConsoleAppender, file::FileAppender},
  config::{Appender, Config, Root},
  encode::pattern::PatternEncoder,
};

pub mod chunk;
pub mod math;
pub mod net;
pub mod registry;
pub mod stream;
pub mod util;

pub use registry::Registry;

pub use generated::{proto, version};

// #[derive(Debug)]
// pub struct KeepAlivePolicy {
//   age: Duration,
// }
//
// impl KeepAlivePolicy {
//   pub fn new(age: Durationn) -> Self {
//     KeepAlivePolicy { age }
//   }
// }
//
// impl Policy for KeepAlivePolicy {
//   fn process(&self, log: &mut LogFile) -> anyhow::Result<()> {
//     if self.trigger.trigger(log)? {
//       log.roll();
//       fs::remove_file(file).map_err(Into::into);
//       self.roller.roll(log.path())?;
//     }
//     Ok(())
//   }
// }

/// Initializes logger. Might do more things in the future.
pub fn init(name: &str) {
  // Put line numbers in debug builds, but not in release builds.
  #[cfg(debug_assertions)]
  let pat = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S:%f)} {f}:{L} [{h({l})}] {m}{n}");
  #[cfg(not(debug_assertions))]
  let pat = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S:%f)} [{h({l})}] {m}{n}");

  let stdout = ConsoleAppender::builder().encoder(Box::new(pat.clone())).build();
  let disk = FileAppender::builder()
    .encoder(Box::new(pat))
    .build(
      format!("log/{}.log", name),
      // Box::new(KeepAlivePolicy::new(Duration::from_secs(60 * 60 * 24 * 7))),
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
