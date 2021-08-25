use common::math::{Pos, RngCore, WyhashRng};

#[derive(Clone, Copy)]
pub struct Line {
  start: Pos,
  end:   Pos,
}

pub struct Traverse {
  line:    Line,
  current: Pos,
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
      line.start = line.start.add_y(100);
      line.end = line.end.add_y(100);
    }
    tree
  }

  fn recursive_add(&mut self, rng: &mut WyhashRng, root: Pos, level: u32, total: u32) {
    if level > rng.next_u32() % total {
      return;
    }
    let mut range_xz = (total - level) * 4;
    let mut range_y = (total - level) / 3;
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
        ((rng.next_u32() % (range_y * 2)) as i32 - range_y as i32) + root.y() / 10 - 1,
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
  pub fn traverse(&self) -> Traverse {
    Traverse { line: *self, current: self.start }
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
    let prev_total_dist = self.line.end().dist_squared(self.current);
    let mut min_line_dist = 1.0;
    let mut min_pos = self.current;
    for offset in [
      Pos::new(1, 0, 0),
      Pos::new(0, 1, 0),
      Pos::new(0, 0, 1),
      Pos::new(-1, 0, 0),
      Pos::new(0, -1, 0),
      Pos::new(0, 0, -1),
    ] {
      let total_dist = self.line.end().dist_squared(self.current + offset);
      if total_dist > prev_total_dist {
        continue;
      }
      let line_dist = self.line.dist_squared(self.current + offset);
      if line_dist < min_line_dist {
        min_line_dist = line_dist;
        min_pos = self.current + offset;
      }
    }
    self.current = min_pos;
    Some(ret)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_dist() {
    let line = Line::new(Pos::new(0, 0, 0), Pos::new(5, 0, 0));
    assert_eq!(line.dist(Pos::new(0, 0, 0)), 0.0);
    assert_eq!(line.dist(Pos::new(1, 0, 0)), 0.0);
    assert_eq!(line.dist(Pos::new(0, 1, 0)), 1.0);
    assert_eq!(line.dist(Pos::new(0, 2, 0)), 2.0);
    assert_eq!(line.dist(Pos::new(1, 1, 0)), 1.0);
    assert_eq!(line.dist(Pos::new(3, 1, 0)), 1.0);
  }

  #[test]
  fn test_traverse() {
    let line = Line::new(Pos::new(0, 0, 0), Pos::new(5, 6, 7));
    for (i, p) in line.traverse().enumerate() {
      dbg!(p);
      if i > 20 {
        panic!("shouldn't take this long!");
      }
    }
    let line = Line::new(Pos::new(1, 2, 6), Pos::new(-5, -6, -7));
    for (i, p) in line.traverse().enumerate() {
      dbg!(p);
      if i > 30 {
        panic!("shouldn't take this long!");
      }
    }
    let line = Line::new(Pos::new(3, 0, 1), Pos::new(6, 0, 2));
    for (i, p) in line.traverse().enumerate() {
      dbg!(p);
      if i > 10 {
        panic!("shouldn't take this long!");
      }
    }
  }
}
