use super::ConnStream;
use crate::cli::LineReader;
use sc_common::net::sb;
use std::{io, io::Write};

pub async fn handle(
  command: &str,
  args: &[&str],
  stream: &mut ConnStream,
  l: &mut LineReader,
) -> io::Result<()> {
  match command {
    "send" => {
      writeln!(l, "running send")?;
    }
    "say" => {
      let saying = args.join(" ");
      writeln!(l, "saying {}", saying)?;
      stream.write(sb::Packet::Chat { message: saying.into() });
    }
    c if c.starts_with('/') => {
      let mut out = command.to_string();
      out.push_str(&args.join(" "));
      writeln!(l, "sending command {}", out)?;
      stream.write(sb::Packet::Chat { message: out });
    }
    _ => {
      writeln!(l, "unknown command: {}", command)?;
    }
  }
  Ok(())
}
