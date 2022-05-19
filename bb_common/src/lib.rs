#![doc(
  html_playground_url = "https://play.rust-lang.org/",
  test(no_crate_inject, attr(deny(warnings)))
)]

#[macro_use]
extern crate log;

// use flexi_logger::{Duplicate, LogTarget, Logger};
#[cfg(feature = "host")]
use log::LevelFilter;

pub mod chunk;
pub mod config;
pub mod math;
pub mod metadata;
pub mod nbt;
#[cfg(feature = "host")]
pub mod net;
pub mod registry;
pub mod util;
pub mod version;

pub use registry::Registry;

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

/*
#[cfg(feature = "host")]
pub fn make_pattern() -> PatternEncoder {
  #[cfg(debug_assertions)]
  let pat = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S:%f)} {f}:{L} [{h({l})}] {m}{n}");
  #[cfg(not(debug_assertions))]
  let pat = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S:%f)} [{h({l})}] {m}{n}");
  pat
}
*/

/// Initializes logger. Might do more things in the future.
pub fn init(name: &str) {
  #[cfg(feature = "host")]
  {
    init_with_level(name, LevelFilter::Info)
  }
  #[cfg(not(feature = "host"))]
  {
    let _ = name;
  }
}

#[cfg(feature = "host")]
pub fn init_with_level(_name: &str, level: LevelFilter) {
  use log::{Level, Metadata, Record};

  struct Logger;

  impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
      metadata.level() <= Level::Info
        && !metadata.target().starts_with("regalloc")
        && !metadata.target().starts_with("wasmer_compiler")
    }

    fn log(&self, record: &Record) {
      if self.enabled(record.metadata()) {
        let now = chrono::Local::now();
        print!("{} ", now.format("%Y-%m-%d %H:%M:%S%.3f"));
        #[cfg(debug_assertions)]
        {
          if let Some(path) = record.module_path() {
            print!("{path}");
          }
          if let Some(line) = record.line() {
            print!(":{line}");
          }
          print!(" ");
        }
        match record.level() {
          Level::Trace => print!("[\x1b[36mTRACE\x1b[0m]"),
          Level::Debug => print!("[\x1b[34mDEBUG\x1b[0m]"),
          Level::Info => print!("[\x1b[32mINFO\x1b[0m]"),
          Level::Warn => print!("[\x1b[33mWARN\x1b[0m]"),
          Level::Error => print!("[\x1b[31m\x1b[1mERROR\x1b[0m]"),
        }
        println!(" {}", record.args());
      }
    }

    fn flush(&self) {}
  }

  static LOGGER: Logger = Logger;
  log::set_logger(&LOGGER).map(|()| log::set_max_level(level)).unwrap();
}
