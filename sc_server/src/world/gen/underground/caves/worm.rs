use crate::{block, world::chunk::MultiChunk};
use sc_common::math::{ChunkPos, FastMath, Pos, RngCore, WyhashRng};
use std::f64::consts::PI;

#[derive(Debug)]
pub struct CaveWorm {
  rng:        WyhashRng,
  pos:        Pos,
  steps:      Vec<Pos>,
  angle_vert: f64,
  angle_horz: f64,
}

impl CaveWorm {
  pub fn new(seed: u64, pos: Pos) -> Self {
    let mut rng = WyhashRng::new(seed);
    let angle_horz = ((rng.next_u32() % 1000) as f64 / 1000.0 - 0.5) * PI; // -PI to PI
    CaveWorm { rng, pos, steps: vec![], angle_vert: 0.0, angle_horz }
  }
  pub fn process(&self, chunk_pos: ChunkPos, c: &mut MultiChunk) {
    for pos in &self.steps {
      if pos.chunk() == chunk_pos {
        // c.set_kind(self.pos.chunk_rel(), block::Kind::LimeWool).unwrap();
        let min = (pos.chunk_rel() - Pos::new(1, 1, 1)).max(Pos::new(0, 0, 0));
        let max = (pos.chunk_rel() + Pos::new(1, 1, 1)).min(Pos::new(15, 255, 15));
        c.fill_kind(min, max, block::Kind::Air).unwrap();
      }
    }
  }

  /// Generates a cave path. This will recursivly spawn children, which will
  /// carve shorter paths. All children's paths will be appended to
  /// `self.steps`.
  pub fn carve(&mut self, offset: u32) {
    let steps = self.rng.next_u32() % 20 + 100;
    if steps < offset {
      return;
    }
    for step in offset..steps {
      self.steps.push(self.pos);
      self.advance();
      if self.rng.next_u32() % 16 == 0 {
        let mut worm = CaveWorm::new(self.rng.next_u64(), self.pos);
        worm.carve(step + 5);
        self.steps.append(&mut worm.steps);
      }
    }
  }

  fn advance(&mut self) {
    let angle_vert_cos = self.angle_vert.fast_cos();
    let direction_x = self.angle_horz.fast_cos() * angle_vert_cos;
    let direction_y = self.angle_vert.fast_sin();
    let direction_z = self.angle_horz.fast_sin() * angle_vert_cos;
    self.pos +=
      Pos::new((direction_x * 3.0) as i32, (direction_y * 3.0) as i32, (direction_z * 3.0) as i32);
    self.pos = self.pos.with_y(self.pos.y().max(0).min(255));
    // -0.8 to 0.2
    self.angle_vert = ((self.rng.next_u32() % 1024) as f64 / 512.0 - 1.0) * 0.5 - 0.6;
    // -0.8 to 0.8
    self.angle_horz += ((self.rng.next_u32() % 1024) as f64 / 512.0 - 1.0) * 0.8;
  }
}
