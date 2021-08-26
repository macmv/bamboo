use crate::{block, world::chunk::MultiChunk};
use common::math::{ChunkPos, FPos, Pos, RngCore, WyhashRng};

struct Vec3 {
  x: f64,
  y: f64,
  z: f64,
}

pub struct CaveWorm {
  rng:       WyhashRng,
  pos:       Pos,
  direction: Vec3,
}

impl CaveWorm {
  pub fn new(seed: u64, pos: Pos) -> Self {
    let mut rng = WyhashRng::new(seed);
    let angle_y = ((rng.next_u32() % 1000) as f64 / 1000.0 - 0.5) * std::f64::consts::PI; // -PI to PI
    let mut direction = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    direction.x = angle_y.cos();
    direction.z = angle_y.sin();
    CaveWorm { rng, pos, direction }
  }
  pub fn carve(&mut self, chunk_pos: ChunkPos, c: &mut MultiChunk, offset: u32) {
    let steps = self.rng.next_u32() % 10 + 20;
    if steps < offset {
      return;
    }
    for step in offset..steps {
      if self.pos.chunk() == chunk_pos {
        // c.set_kind(self.pos.chunk_rel(), block::Kind::LimeWool).unwrap();
        let mut min = self.pos.chunk_rel() - Pos::new(1, 1, 1);
        let mut max = self.pos.chunk_rel() + Pos::new(1, 1, 1);
        if min.x() < 0 {
          min = min.with_x(0);
        }
        if min.y() < 0 {
          min = min.with_y(0);
        }
        if min.z() < 0 {
          min = min.with_z(0);
        }
        if max.x() > 15 {
          max = max.with_x(15);
        }
        if max.y() < 0 {
          max = max.with_y(0);
        }
        if max.z() > 15 {
          max = max.with_z(15);
        }
        c.fill_kind(min, max, block::Kind::Air).unwrap();
      }
      self.advance();
      if self.rng.next_u32() % 16 == 0 {
        CaveWorm::new(self.rng.next_u64(), self.pos).carve(chunk_pos, c, step);
      }
    }
  }

  fn advance(&mut self) {
    self.pos += Pos::new(
      (self.direction.x * 3.0) as i32,
      (self.direction.y * 3.0) as i32,
      (self.direction.z * 3.0) as i32,
    );
    if self.pos.y() > 255 {
      self.pos = self.pos.with_y(255);
    }
    if self.pos.y() < 0 {
      self.pos = self.pos.with_y(0);
    }
    let angle_vert = ((self.rng.next_u32() % 1024) as f64 / 1024.0) * 0.6 - 0.5; // -0.5 to 0.1
    self.direction.y = angle_vert.sin();

    let mut angle_y = (self.direction.z / self.direction.x).atan();
    angle_y += ((self.rng.next_u32() % 1024) as f64 / 1024.0 - 0.5) * 0.8; // -0.8 to 0.8

    // Need to divide, to keep the vector normalized
    self.direction.x = angle_y.cos() / angle_vert.cos();
    self.direction.z = angle_y.sin() / angle_vert.cos();
  }
}
