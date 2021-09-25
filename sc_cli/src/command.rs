use super::ConnWriter;
use crate::cli::LineReader;
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
    _ => {
      writeln!(l, "unknown command: {}", command)?;
    }
  }
  Ok(())
}
