use super::ConnStream;
use crate::cli::LineReader;
use sc_proxy::gnet::sb;
use std::{io, io::Write};

pub fn handle(
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
      stream.write(sb::Packet::ChatV8 { message: saying });
    }
    "move" => {
      if args.len() != 3 {
        writeln!(l, "expected a position")?;
        return Ok(());
      }
      let x = match args[0].parse() {
        Ok(v) => v,
        Err(_) => return Ok(()),
      };
      let y = match args[1].parse() {
        Ok(v) => v,
        Err(_) => return Ok(()),
      };
      let z = match args[2].parse() {
        Ok(v) => v,
        Err(_) => return Ok(()),
      };
      writeln!(l, "moving to {} {} {}", x, y, z)?;
      stream.write(sb::Packet::PlayerPositionV8 { x, y, z, on_ground: true });
    }
    c if c.starts_with('/') => {
      let mut out = command.to_string();
      out.push_str(&args.join(" "));
      writeln!(l, "sending command {}", out)?;
      stream.write(sb::Packet::ChatV8 { message: out });
    }
    _ => {
      writeln!(l, "unknown command: {}", command)?;
    }
  }
  Ok(())
}
