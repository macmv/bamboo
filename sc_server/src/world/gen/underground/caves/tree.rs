use sc_common::math::{terrain::Point, ChunkPos, Pos, RngCore, WyhashRng};

#[derive(Clone, Copy, Debug)]
pub struct Line {
  start: Pos,
  end:   Pos,
}

#[derive(Debug)]
pub struct Traverse {
  line:          Line,
  current:       Pos,
  entered_chunk: bool,
  offset:        Point,
  chunk:         ChunkPos,
}

#[derive(Clone)]
pub struct CaveTree {
  lines: Vec<Line>,
}

impl CaveTree {
  pub fn new(seed: u64) -> Self {
    let mut tree = CaveTree { lines: vec![] };
    let mut rng = WyhashRng::new(seed);
    tree.recursive_add(&mut rng, Pos::new(0, 0, 0), 1, 8);
    for line in tree.lines.iter_mut() {
      line.start = line.start.add_y(20);
      line.end = line.end.add_y(20);
    }
    tree
  }

  fn recursive_add(&mut self, rng: &mut WyhashRng, root: Pos, level: u32, total: u32) {
    if level > rng.next_u32() % total {
      return;
    }
    let mut range_xz = (total - level) * 8;
    let mut range_y = (total - level) * 2;
    if range_xz == 0 {
      range_xz = 1
    }
    if range_y == 0 {
      range_y = 1
    }
    let next = root
      + Pos::new(
        // Adding some of the root makes the caves spread outwards more
        ((rng.next_u32() % (range_xz * 2)) as i32 - range_xz as i32) + root.x() / 10,
        ((rng.next_u32() % (range_y * 2)) as i32 - range_y as i32) + root.y() / 10 - 3,
        ((rng.next_u32() % (range_xz * 2)) as i32 - range_xz as i32) + root.z() / 10,
      );
    self.lines.push(Line::new(root, next));
    for _ in 0..(rng.next_u32() % (total - level)) {
      self.recursive_add(rng, next, level + 1, total);
    }
  }

  pub fn lines(&self) -> &[Line] {
    &self.lines
  }
}

impl Line {
  pub fn new(start: Pos, end: Pos) -> Self {
    Line { start, end }
  }
  #[inline(always)]
  pub fn start(&self) -> Pos {
    self.start
  }
  #[inline(always)]
  pub fn end(&self) -> Pos {
    self.end
  }
  pub fn traverse(&self, offset: Point, chunk: ChunkPos) -> Traverse {
    Traverse { line: *self, current: self.start, entered_chunk: false, offset, chunk }
  }

  /// Returns the squared distance to the line, as if the line were infinitely
  /// long.
  pub fn dist_squared(&self, pos: Pos) -> f64 {
    let dist = self.start().dist(self.end());
    // Line direction, normalized
    let dir_x = (self.start().x() - self.end().x()) as f64 / dist;
    let dir_y = (self.start().y() - self.end().y()) as f64 / dist;
    let dir_z = (self.start().z() - self.end().z()) as f64 / dist;
    // Difference between pos and start
    let v_x = (pos.x() - self.start().x()) as f64;
    let v_y = (pos.y() - self.start().y()) as f64;
    let v_z = (pos.z() - self.start().z()) as f64;
    // Dot product
    let dot = v_x * dir_x + v_y * dir_y + v_z * dir_z;
    // This is the closest point on the line
    let nearest_x = self.start().x() as f64 + dir_x * dot;
    let nearest_y = self.start().y() as f64 + dir_y * dot;
    let nearest_z = self.start().z() as f64 + dir_z * dot;

    (pos.x() as f64 - nearest_x).powi(2)
      + (pos.y() as f64 - nearest_y).powi(2)
      + (pos.z() as f64 - nearest_z).powi(2)
  }
}

impl Iterator for Traverse {
  type Item = Pos;

  fn next(&mut self) -> Option<Pos> {
    if self.current == self.line.end() {
      return None;
    }
    let ret = self.current;
    let dx = self.line.end().x() - self.current.x();
    let dy = self.line.end().y() - self.current.y();
    let dz = self.line.end().z() - self.current.z();
    let len = ((dx.pow(2) + dy.pow(2) + dz.pow(2)) as f64).sqrt();
    let dx = dx as f64 / len * 1.8;
    let dy = dy as f64 / len * 1.8;
    let dz = dz as f64 / len * 1.8;
    self.current = self.current + Pos::new(dx as i32, dy as i32, dz as i32);
    // This function is error prone, this helps debug the server freezing on this
    // function.
    debug_assert_ne!(
      self.current, ret,
      "couldn't update postition in traverse! line: {} to {}, current: {}",
      self.line.start, self.line.end, self.current
    );
    let ret = Pos::new(ret.x() + self.offset.x, ret.y(), ret.z() + self.offset.y);
    // A straight line will always enter a chunk a most one time. So we can use that
    // to skip all the extra iterating after we leave.
    if !self.entered_chunk && ret.chunk() == self.chunk {
      self.entered_chunk = true;
      Some(ret)
    } else if self.entered_chunk && ret.chunk() != self.chunk {
      self.current = self.line.end();
      None
    } else {
      Some(ret)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_dist() {
    let line = Line::new(Pos::new(0, 0, 0), Pos::new(5, 0, 0));
    assert_eq!(line.dist_squared(Pos::new(0, 0, 0)), 0.0_f64.powi(2));
    assert_eq!(line.dist_squared(Pos::new(1, 0, 0)), 0.0_f64.powi(2));
    assert_eq!(line.dist_squared(Pos::new(0, 1, 0)), 1.0_f64.powi(2));
    assert_eq!(line.dist_squared(Pos::new(0, 2, 0)), 2.0_f64.powi(2));
    assert_eq!(line.dist_squared(Pos::new(1, 1, 0)), 1.0_f64.powi(2));
    assert_eq!(line.dist_squared(Pos::new(3, 1, 0)), 1.0_f64.powi(2));
  }

  #[test]
  fn test_traverse() {
    let line = Line::new(Pos::new(0, 0, 0), Pos::new(5, 6, 7));
    for (i, p) in line.traverse(Point::new(0, 0), ChunkPos::new(0, 0)).enumerate() {
      dbg!(p);
      if i > 20 {
        panic!("shouldn't take this long!");
      }
    }
    let line = Line::new(Pos::new(1, 2, 6), Pos::new(-5, -6, -7));
    for (i, p) in line.traverse(Point::new(0, 0), ChunkPos::new(0, 0)).enumerate() {
      dbg!(p);
      if i > 30 {
        panic!("shouldn't take this long!");
      }
    }
    let line = Line::new(Pos::new(3, 0, 1), Pos::new(6, 0, 2));
    for (i, p) in line.traverse(Point::new(0, 0), ChunkPos::new(0, 0)).enumerate() {
      dbg!(p);
      if i > 10 {
        panic!("shouldn't take this long!");
      }
    }
  }
}
