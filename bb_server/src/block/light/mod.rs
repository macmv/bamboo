use bb_common::{
  chunk::{light::LightPropagator, Chunk, LightChunk, Section},
  math::RelPos,
  util::Face,
};

#[cfg(test)]
mod test;

/// Marker trait, which will propagate block light information.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockLight {}
/// Marker trait, which will propagate sky light information.
#[derive(Debug, Clone, PartialEq)]
pub struct SkyLight {}

impl LightPropagator for BlockLight {
  fn initial_level() -> u8 { 0 }
  fn propagate<P: LightPropagator, S: Section>(
    light: &mut LightChunk<P>,
    chunk: &Chunk<S>,
    pos: RelPos,
  ) {
    let directions = [Face::Top, Face::Bottom, Face::North, Face::South, Face::East, Face::West];
    let level = light.get_light(pos);
    let mut queue = vec![(pos, level)];
    let mut other_queue = vec![];
    while !queue.is_empty() {
      for &(source, mut level) in &queue {
        for dir in directions {
          let new_pos = match source.checked_add(dir) {
            Some(p) => p,
            None => continue,
          };
          if new_pos.y() < 0 || new_pos.y() > 255 {
            continue;
          }
          if chunk.get_block(new_pos).unwrap() == 0 {
            let other_level = light.get_light(new_pos);
            if other_level > level + 1 {
              println!("CURRENT IS TOO DIM");
              // The current block is too dim, so fix it, and queue the
              // neighboring block.
              level = other_level - 1;
              light.set_light(pos, other_level - 1);
              other_queue.push((new_pos, other_level));
            }
          }
        }
        for dir in directions {
          let new_pos = match source.checked_add(dir) {
            Some(p) => p,
            None => continue,
          };
          if new_pos.y() < 0 || new_pos.y() > 255 {
            continue;
          }
          if chunk.get_block(new_pos).unwrap() == 0 {
            let other_level = light.get_light(new_pos);
            if level >= 1 && other_level < level - 1 {
              println!("NEIGHBOR IS TOO DIM");
              // The neighbor is too dim, queue `new_pos` to be updated.
              light.set_light(new_pos, level - 1);
              other_queue.push((new_pos, level - 1));
            }
          }
        }
      }
      queue.clear();
      std::mem::swap(&mut queue, &mut other_queue);
    }
  }
  fn propagate_all<P: LightPropagator, S: Section>(light: &mut LightChunk<P>, chunk: &Chunk<S>) {
    let _ = (light, chunk);
    // TODO
  }
}

impl LightPropagator for SkyLight {
  fn initial_level() -> u8 { 15 }
  fn propagate<P: LightPropagator, S: Section>(
    light: &mut LightChunk<P>,
    chunk: &Chunk<S>,
    pos: RelPos,
  ) {
    let directions = [Face::Top, Face::Bottom, Face::North, Face::South, Face::East, Face::West];
    let level = light.get_light(pos);
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
          if chunk.get_block(new_pos).unwrap() == 0 {
            if dir == Face::Bottom {
              other_queue.push((new_pos, level));
            } else if light.get_light(new_pos) < level - 1 {
              light.set_light(new_pos, level - 1);
              other_queue.push((new_pos, level));
            }
          }
        }
      }
      queue.clear();
      std::mem::swap(&mut queue, &mut other_queue);
    }
  }
  fn propagate_all<P: LightPropagator, S: Section>(light: &mut LightChunk<P>, chunk: &Chunk<S>) {
    let _ = (light, chunk);
    // TODO
  }
}
