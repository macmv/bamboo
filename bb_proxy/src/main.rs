#[macro_use]
extern crate log;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
  /// Writes the default config to `proxy-default.toml`. Does not overwrite
  /// the existing config.
  #[clap(long)]
  write_default_config: bool,
}

fn main() {
  /*
  use pprof::protos::Message;
  use std::{fs::File, io::Write};

  let profile = true;
  let guard = if profile {
    println!("starting cpu profiler");
    Some(pprof::ProfilerGuard::new(100).unwrap())
  } else {
    None
  };
  */

  let args = Args::parse();
  let config = if args.write_default_config {
    bb_proxy::load_config_write_default("proxy.toml", "proxy-default.toml")
  } else {
    bb_proxy::load_config("proxy.toml")
  };

  match bb_proxy::run(config) {
    Ok(_) => (),
    Err(e) => error!("error: {}", e),
  }

  /*
  if let Some(guard) = guard {
    match guard.report().build() {
      Ok(report) => {
        let mut file = File::create("pprof.pb").unwrap();
        let profile = report.pprof().unwrap();

        let mut content = Vec::new();
        profile.encode(&mut content).unwrap();
        file.write_all(&content).unwrap();
      }
      Err(e) => {
        println!("failed to generate report: {}", e);
      }
    };
  }
  */
}
