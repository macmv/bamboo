use bb_common::{
  chunk::LightChunk,
  math::{Pos, RelPos},
  util::Face,
};
use std::{cmp, collections::VecDeque, ops::BitOr};

use crate::block::{self, light::BlockLightChunk};

use super::{BlockData, MultiChunk};

pub struct LightPropogator {
  increase_queue: VecDeque<Increase>,
  decrease_queue: VecDeque<Decrease>,
}

struct ChunkPropogator<'a> {
  // TODO: This should be &BlockData, but its &mut BlockData for debugging.
  block: &'a mut BlockData,
  prop:  &'a mut LightPropogator,
  light: &'a mut LightChunk,
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

// TODO: Re-enable vertical light propogation
// const ALL_DIRS: Dirs = Dirs(0b00111111);
const ALL_DIRS: Dirs = Dirs(0b00111100);

impl LightPropogator {
  pub fn new() -> Self {
    LightPropogator { increase_queue: VecDeque::new(), decrease_queue: VecDeque::new() }
  }

  fn chunk<'a>(
    &'a mut self,
    chunk: &'a mut BlockData,
    light: &'a mut BlockLightChunk,
  ) -> ChunkPropogator<'a> {
    ChunkPropogator { block: chunk, prop: self, light: &mut light.data }
  }
}

fn opacity(kind: block::Kind) -> u8 {
  match kind {
    block::Kind::Air => 0,
    block::Kind::Torch => 0,
    _ => 15,
  }
}

impl ChunkPropogator<'_> {
  fn set_opacity(&mut self, pos: RelPos, new_opacity: u8) {
    if new_opacity == 15 {
      self.decrease_block_light(pos);
    } else {
      for face in ALL_DIRS.iter() {
        let Some(neighbor) = pos.checked_add(face) else { continue };
        let current_level = self.light.get_light(neighbor);

        // TODO: Use a block converter
        let opacity = opacity(self.block.get_kind(neighbor).unwrap());
        let Some(target_level) = current_level.checked_sub(cmp::max(1, opacity)) else { continue };

        if target_level > self.light.get_light(pos) {
          self.set(pos, target_level);
        }
      }

      self.prop.increase_queue.push_back(Increase {
        pos,
        level: self.light.get_light(pos),
        dirs: ALL_DIRS,
        recheck: true,
      });

      self.propogate_increase()
    }
  }

  fn set_light(&mut self, pos: RelPos, level: u8) {
    if level > 0 {
      self.increase_block_light(pos, level)
    } else {
      self.decrease_block_light(pos)
    }
  }

  fn set(&mut self, pos: RelPos, level: u8) {
    self.light.set_light(pos, level);

    // This is to debug lights in the debug world.
    self
      .block
      .set_kind(
        pos.add_y(-1),
        match level {
          5 => block::Kind::LightBlueWool,
          4 => block::Kind::LimeWool,
          3 => block::Kind::YellowWool,
          2 => block::Kind::OrangeWool,
          1 => block::Kind::RedWool,
          _ => block::Kind::WhiteWool,
        },
      )
      .unwrap();
  }

  fn increase_block_light(&mut self, pos: RelPos, level: u8) {
    if level > 15 {
      panic!("invalid light level {level}");
    }

    let existing_level = self.light.get_light(pos);
    if existing_level < level {
      self.prop.increase_queue.push_back(Increase { pos, level, dirs: ALL_DIRS, recheck: false });
      self.set(pos, level);

      self.propogate_increase()
    }
  }

  fn decrease_block_light(&mut self, pos: RelPos) {
    let existing_level = self.light.get_light(pos);
    if existing_level > 0 {
      self.prop.decrease_queue.push_back(Decrease { pos, level: existing_level });
      self.set(pos, 0);

      self.propogate_decrease();
    }
  }

  fn propogate_increase(&mut self) {
    while let Some(increase) = self.prop.increase_queue.pop_front() {
      if increase.recheck && self.light.get_light(increase.pos) != increase.level {
        continue;
      }

      for face in increase.dirs {
        let Some(neighbor) = increase.pos.checked_add(face) else { continue };
        let current_level = self.light.get_light(neighbor);

        // TODO: Use a block converter
        let opacity = opacity(self.block.get_kind(neighbor).unwrap());
        let Some(target_level) = increase.level.checked_sub(cmp::max(1, opacity)) else { continue };

        if target_level <= current_level {
          continue;
        }

        self.set(neighbor, target_level);

        if target_level > 1 && target_level != current_level {
          self.prop.increase_queue.push_back(Increase {
            pos:     neighbor,
            level:   target_level,
            // all except the opposite of the face we came from
            dirs:    Dirs(ALL_DIRS.0 & !Dirs::from_face(face.opposite()).0),
            recheck: increase.recheck,
          });
        }
      }
    }
  }

  fn propogate_decrease(&mut self) {
    while let Some(decrease) = self.prop.decrease_queue.pop_front() {
      for face in ALL_DIRS.iter() {
        let Some(neighbor) = decrease.pos.checked_add(face) else { continue };
        let current_level = self.light.get_light(neighbor);

        if current_level == 0 {
          continue;
        }

        // TODO: Use a block converter
        let opacity = opacity(self.block.get_kind(neighbor).unwrap());
        let target_level = decrease.level.saturating_sub(cmp::max(1, opacity));

        if current_level > target_level {
          self.prop.increase_queue.push_back(Increase {
            pos:     neighbor,
            level:   current_level,
            dirs:    ALL_DIRS,
            recheck: true,
          });
          continue;
        }

        // TODO: Use a block converter
        let emitted =
          if self.block.get_kind(neighbor).unwrap() == block::Kind::Torch { 5 } else { 0 };
        if emitted > 0 {
          self.prop.increase_queue.push_back(Increase {
            pos:     neighbor,
            level:   emitted,
            dirs:    ALL_DIRS,
            recheck: false,
          })
        }

        self.set(neighbor, 0);

        if target_level > 0 {
          self.prop.decrease_queue.push_back(Decrease { pos: neighbor, level: target_level });
        }
      }
    }

    // re-populate any sources we found
    self.propogate_increase();
  }
}

impl super::World {
  pub(crate) fn light_update(&self, pos: Pos, old_ty: block::Type, new_ty: block::Type) {
    let old_data = self.block_converter().get(old_ty.kind());
    let new_data = self.block_converter().get(new_ty.kind());

    self.chunk(pos.chunk(), |mut c| {
      let c: &mut MultiChunk = &mut c;
      let mut light = self.block_light.lock();
      let mut chunk_prop = light.chunk(&mut c.block, &mut c.block_light);

      // TODO: `data.transparent` doesn't work :/
      let old_opacity = opacity(old_ty.kind());
      let new_opacity = opacity(new_ty.kind());

      if old_data.emit_light > 0 || new_data.emit_light > 0 {
        if new_ty.kind() == block::Kind::Torch {
          chunk_prop.set_light(pos.chunk_rel(), 5);
        } else {
          chunk_prop.set_light(pos.chunk_rel(), new_data.emit_light);
        }
      } else if old_opacity != new_opacity {
        chunk_prop.set_opacity(pos.chunk_rel(), new_opacity);
      }
    });
  }
}

impl BitOr for Dirs {
  type Output = Self;

  fn bitor(self, rhs: Self) -> Self::Output { Dirs(self.0 | rhs.0) }
}

impl Dirs {
  pub fn contains(&self, dir: Self) -> bool { self.0 & dir.0 != 0 }
  pub fn iter(&self) -> DirsIter { DirsIter { dirs: *self, i: 0 } }

  pub const fn from_face(face: Face) -> Dirs {
    match face {
      Face::Top => UP,
      Face::Bottom => DOWN,
      Face::North => NORTH,
      Face::South => SOUTH,
      Face::East => EAST,
      Face::West => WEST,
    }
  }
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
