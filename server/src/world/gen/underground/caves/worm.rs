use crate::{block, world::chunk::MultiChunk};
use common::math::{ChunkPos, FastMath, Pos, RngCore, WyhashRng};

#[derive(Debug)]
struct Vec3 {
  x: f64,
  y: f64,
  z: f64,
}

#[derive(Debug)]
pub struct CaveWorm {
  rng:        WyhashRng,
  pos:        Pos,
  steps:      Vec<Pos>,
  // direction: Vec3,
  angle_vert: f64,
  angle_horz: f64,

  children: Vec<CaveWorm>,
}

impl CaveWorm {
  pub fn new(seed: u64, pos: Pos) -> Self {
    let mut rng = WyhashRng::new(seed);
    let angle_horz = ((rng.next_u32() % 1000) as f64 / 1000.0 - 0.5) * std::f64::consts::PI; // -PI to PI
    CaveWorm { rng, pos, steps: vec![], angle_vert: 0.0, angle_horz, children: vec![] }
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
    for child in &self.children {
      child.process(chunk_pos, c);
    }
  }

  /// Generates a cave path. This will recursivly add children, which will carve
  /// shorter paths.
  pub fn carve(&mut self, offset: u32) {
    let steps = self.rng.next_u32() % 10 + 20;
    if steps < offset {
      return;
    }
    for step in offset..steps {
      self.steps.push(self.pos);
      self.advance();
      if self.rng.next_u32() % 16 == 0 {
        let mut worm = CaveWorm::new(self.rng.next_u64(), self.pos);
        worm.carve(step + 5);
        self.children.push(worm);
      }
    }
  }

  fn advance(&mut self) {
    let angle_vert_cos = self.angle_vert.fast_cos();
    let direction_x = self.angle_horz.fast_cos() / angle_vert_cos;
    let direction_y = self.angle_vert.fast_sin();
    let direction_z = self.angle_horz.fast_sin() / angle_vert_cos;
    self.pos +=
      Pos::new((direction_x * 3.0) as i32, (direction_y * 3.0) as i32, (direction_z * 3.0) as i32);
    if self.pos.y() > 255 {
      self.pos = self.pos.with_y(255);
    }
    if self.pos.y() < 0 {
      self.pos = self.pos.with_y(0);
    }
    // -0.5 to 0.1
    self.angle_vert = ((self.rng.next_u32() % 1024) as f64 / 1024.0) * 0.6 - 0.5;
    // -0.8 to 0.8
    self.angle_horz += ((self.rng.next_u32() % 1024) as f64 / 1024.0 - 0.5) * 0.8;
  }
}
