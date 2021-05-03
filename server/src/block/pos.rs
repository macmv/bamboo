use std::{error::Error, fmt};

#[derive(Debug)]
pub struct Pos {
  x: i32,
  y: i32,
  z: i32,
}

impl Pos {}
impl fmt::Display for Pos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "pos {{ x: {}, y: {}, z: {} }}", self.x, self.y, self.z)
  }
}

#[derive(Debug)]
pub struct PosError {
  pos: Pos,
  msg: String,
}

impl fmt::Display for PosError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid position: {} {}", self.pos, self.msg)
  }
}

impl Error for PosError {}
