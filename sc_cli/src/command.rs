use super::ConnWriter;
use crate::cli::LineReader;
use sc_common::net::sb;
use std::{io, io::Write};

pub async fn handle(
  command: &str,
  args: &[&str],
  w: &mut ConnWriter,
  l: &mut LineReader,
) -> io::Result<()> {
  match command {
    "send" => {
      writeln!(l, "running send")?;
    }
    "say" => {
      let saying = args.join(" ");
      writeln!(l, "saying {}", saying)?;
      w.write(sb::Packet::Chat { message: saying.into() }).await?;
      w.flush().await?;
    }
    c if c.starts_with('/') => {
      let mut out = command.to_string();
      out.push_str(&args.join(" "));
      writeln!(l, "sending command {}", out)?;
      w.write(sb::Packet::Chat { message: out }).await?;
      w.flush().await?;
    }
    _ => {
      writeln!(l, "unknown command: {}", command)?;
    }
  }
  Ok(())
}
