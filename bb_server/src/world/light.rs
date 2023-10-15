use bb_common::{
  math::{Pos, RelPos},
  util::Face,
};
use std::{collections::VecDeque, ops::BitOr};

use crate::block::{self, light::BlockLightChunk};

use super::{BlockData, MultiChunk};

pub struct LightPropogator {
  increase_queue: VecDeque<Increase>,
  decrease_queue: VecDeque<Decrease>,
}

struct Increase {
  pos:     RelPos,
  level:   u8,
  dirs:    Dirs,
  recheck: bool,
}

struct Decrease {
  pos:   RelPos,
  level: u8,
}

/// Stores a bitmask about what directions to go in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dirs(u8);

const UP: Dirs = Dirs(0b00000001);
const DOWN: Dirs = Dirs(0b00000010);
const NORTH: Dirs = Dirs(0b00000100);
const SOUTH: Dirs = Dirs(0b00001000);
const EAST: Dirs = Dirs(0b00010000);
const WEST: Dirs = Dirs(0b00100000);
const ALL_DIRS: Dirs = Dirs(0b00111111);

impl LightPropogator {
  pub fn new() -> Self {
    LightPropogator { increase_queue: VecDeque::new(), decrease_queue: VecDeque::new() }
  }

  pub fn increase(&self, chunk: &BlockData, light: &mut BlockLightChunk, pos: RelPos, level: u8) {}
}

impl super::World {
  pub(crate) fn light_update(&self, pos: Pos, old_ty: block::Type, new_ty: block::Type) {
    let old_data = self.block_converter().get(old_ty.kind());
    let new_data = self.block_converter().get(new_ty.kind());

    if new_data.emit_light > 0 {
      self.chunk(pos.chunk(), |mut c| {
        let c: &mut MultiChunk = &mut c;
        self.block_light.lock().increase(
          &c.block,
          &mut c.block_light,
          pos.chunk_rel(),
          new_data.emit_light,
        )
      });
    } else {
      /*
      match (old_data.transparent, new_data.transparent) {
        (false, true) => self.chunk(pos.chunk(), |c| self.block_light.lock().decrease(pos, old_ty)),
        (false, false) => {}
        (true, true) => {}
        (true, false) => {}
      }
      */
    }
  }
}

impl BitOr for Dirs {
  type Output = Self;

  fn bitor(self, rhs: Self) -> Self::Output { Dirs(self.0 | rhs.0) }
}

impl Dirs {
  pub fn contains(&self, dir: Self) -> bool { self.0 & dir.0 != 0 }
  pub fn iter(&self) -> DirsIter { DirsIter { dirs: *self, i: 0 } }
}

struct DirsIter {
  dirs: Dirs,
  i:    u8,
}

impl IntoIterator for Dirs {
  type Item = Face;
  type IntoIter = DirsIter;

  fn into_iter(self) -> Self::IntoIter { self.iter() }
}

impl Iterator for DirsIter {
  type Item = Face;

  fn next(&mut self) -> Option<Self::Item> {
    let (dir, face) = match self.i {
      0 => (UP, Face::Top),
      1 => (DOWN, Face::Bottom),
      2 => (NORTH, Face::North),
      3 => (SOUTH, Face::South),
      4 => (EAST, Face::East),
      5 => (WEST, Face::West),
      _ => return None,
    };
    self.i += 1;
    if self.dirs.contains(dir) {
      Some(face)
    } else {
      self.next()
    }
  }
}
