use crate::{world::BlockData};
use bb_common::{chunk::LightChunk, math::RelPos, util::Face};

#[cfg(test)]
mod test;

/// Marker trait, which will propagate block light information.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockLightChunk {
  pub data: LightChunk,
}
/// Marker trait, which will propagate sky light information.
#[derive(Debug, Clone, PartialEq)]
pub struct SkyLightChunk {
  pub data: LightChunk,
}

impl BlockLightChunk {
  pub fn new() -> Self { BlockLightChunk { data: LightChunk::new() } }

  pub fn update(&mut self, chunk: &BlockData, pos: RelPos) {
    // TODO: Remove
    let _ = (chunk, pos);
  }
  pub fn update_all(&mut self, chunk: &BlockData) {
    let _ = chunk;

    // make sure there are light sections present.
    //
    // TODO: Remove
    for y in 0..70 {
      self.data.set_light(RelPos::new(0, y, 0), 0);
    }
  }
}

impl SkyLightChunk {
  pub fn new() -> Self { SkyLightChunk { data: LightChunk::new() } }

  pub fn update(&mut self, chunk: &BlockData, pos: RelPos) {
    let directions = [Face::Top, Face::Bottom, Face::North, Face::South, Face::East, Face::West];
    let level = self.data.get_light(pos);
    let mut queue = vec![(pos, level)];
    let mut other_queue = vec![];
    while !queue.is_empty() {
      for &(source, level) in &queue {
        if level == 0 {
          continue;
        }
        for dir in directions {
          let new_pos = match source.checked_add(dir) {
            Some(p) => p,
            None => continue,
          };
          if new_pos.y() < 0 || new_pos.y() > 255 {
            continue;
          }
          let data = chunk.wm().block_converter().get(chunk.get_kind(new_pos).unwrap());
          if data.transparent {
            if dir == Face::Bottom {
              other_queue.push((new_pos, level));
            } else if self.data.get_light(new_pos) < level - 1 {
              self.data.set_light(new_pos, level - 1);
              other_queue.push((new_pos, level));
            }
          }
        }
      }
      queue.clear();
      std::mem::swap(&mut queue, &mut other_queue);
    }
  }
  pub fn update_all(&mut self, chunk: &BlockData) {
    let _ = chunk;
    // TODO
  }
}
