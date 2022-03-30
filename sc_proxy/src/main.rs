#[macro_use]
extern crate log;

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

  match sc_proxy::run() {
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
