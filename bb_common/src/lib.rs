#![doc(
  html_playground_url = "https://play.rust-lang.org/",
  test(no_crate_inject, attr(deny(warnings)))
)]

#[macro_use]
extern crate log;

// use flexi_logger::{Duplicate, LogTarget, Logger};

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

pub use bb_transfer as transfer;
#[cfg(feature = "host")]
pub use flate2;

// #[derive(Debug)]
// pub struct KeepAlivePolicy {
//   age: Duration,
// }
//
// impl KeepAlivePolicy {
//   pub fn new(age: Duration) -> Self {
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

#[cfg(feature = "host")]
use log::LevelFilter;
#[cfg(feature = "host")]
use std::io;

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
  init_with_level_writer(_name, level, io::stdout());
}

#[cfg(feature = "host")]
pub fn init_with_writer(_name: &str, writer: impl std::io::Write + Send + Sync + 'static) {
  init_with_level_writer(_name, LevelFilter::Info, writer);
}

#[cfg(feature = "host")]
pub fn init_with_level_writer<W: std::io::Write + Send + Sync + 'static>(
  _name: &str,
  level: LevelFilter,
  writer: W,
) {
  use log::{Level, Metadata, Record};
  use parking_lot::Mutex;

  #[cfg(unix)]
  let isatty = unsafe { libc::isatty(libc::STDOUT_FILENO) } != 0;
  #[cfg(not(unix))]
  let isatty = false;

  struct Logger<W> {
    writer: Mutex<W>,
    color:  bool,
  }

  impl<W: io::Write> Logger<W> {
    fn log_inner(&self, record: &Record) -> io::Result<()> {
      #[cfg(not(feature = "utclogs"))]
      let now = chrono::Local::now();
      #[cfg(feature = "utclogs")]
      let now = chrono::Utc::now();

      let mut w = self.writer.lock();
      write!(w, "{} ", now.format("%Y-%m-%d %H:%M:%S%.3f"))?;
      #[cfg(debug_assertions)]
      {
        if let Some(path) = record.module_path() {
          write!(w, "{path}")?;
        }
        if let Some(line) = record.line() {
          write!(w, ":{line}")?;
        }
        write!(w, " ")?;
      }
      if self.color {
        match record.level() {
          Level::Trace => write!(w, "[\x1b[36mTRACE\x1b[0m]")?,
          Level::Debug => write!(w, "[\x1b[34mDEBUG\x1b[0m]")?,
          Level::Info => write!(w, "[\x1b[32mINFO\x1b[0m]")?,
          Level::Warn => write!(w, "[\x1b[33mWARN\x1b[0m]")?,
          Level::Error => write!(w, "[\x1b[31m\x1b[1mERROR\x1b[0m]")?,
        }
      } else {
        match record.level() {
          Level::Trace => write!(w, "[TRACE]")?,
          Level::Debug => write!(w, "[DEBUG]")?,
          Level::Info => write!(w, "[INFO]")?,
          Level::Warn => write!(w, "[WARN]")?,
          Level::Error => write!(w, "[ERROR]")?,
        }
      }
      writeln!(w, " {}", record.args())?;
      Ok(())
    }
  }

  impl<W: io::Write + Send + Sync> log::Log for Logger<W> {
    fn enabled(&self, metadata: &Metadata) -> bool {
      !metadata.target().starts_with("regalloc")
        && !metadata.target().starts_with("wasmer_compiler")
        && !metadata.target().starts_with("cranelift")
    }

    fn log(&self, record: &Record) {
      if self.enabled(record.metadata()) {
        let _ = self.log_inner(record);
      }
    }

    fn flush(&self) {}
  }

  // static LOGGER: Logger = Logger;
  // log::set_logger(&LOGGER).map(|()| log::set_max_level(level)).unwrap();
  log::set_boxed_logger(Box::new(Logger { writer: Mutex::new(writer), color: isatty }))
    .map(|()| log::set_max_level(level))
    .unwrap();
}
