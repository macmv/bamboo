use common::math::Pos;

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
    CaveTree {
      lines: vec![
        Line::new(Pos::new(0, 80, 0), Pos::new(5, 80, 10)),
        Line::new(Pos::new(5, 80, 10), Pos::new(15, 80, 5)),
        Line::new(Pos::new(0, 80, 0), Pos::new(-5, 80, 10)),
        Line::new(Pos::new(0, 80, 0), Pos::new(5, 80, 10)),
      ],
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

  /// Returns the distance to the line, as if the line were infinitely long.
  pub fn dist(&self, pos: Pos) -> f64 {
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

    (((pos.x() as f64 - nearest_x).powi(2)
      + (pos.y() as f64 - nearest_y).powi(2)
      + (pos.z() as f64 - nearest_z).powi(2)) as f64)
      .sqrt()
  }
}

impl Iterator for Traverse {
  type Item = Pos;

  fn next(&mut self) -> Option<Pos> {
    if self.current == self.line.end() {
      return None;
    }
    let ret = self.current;
    let prev_total_dist = self.line.end().dist(self.current);
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
      let total_dist = self.line.end().dist(self.current + offset);
      if total_dist > prev_total_dist {
        continue;
      }
      let line_dist = self.line.dist(self.current + offset);
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
