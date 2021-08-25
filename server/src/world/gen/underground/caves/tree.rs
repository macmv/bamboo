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
}

impl Iterator for Traverse {
  type Item = Pos;

  fn next(&mut self) -> Option<Pos> {
    if self.current == self.line.end() {
      return None;
    }
    let ret = self.current;
    let dist = self.line.end().dist(self.current);
    let dx = (self.line.end().x() - self.current.x()) as f64 / dist;
    let dy = (self.line.end().y() - self.current.y()) as f64 / dist;
    let dz = (self.line.end().z() - self.current.z()) as f64 / dist;
    if dx.abs() >= dy.abs() && dx.abs() >= dz.abs() {
      self.current = self.current.add_x(dx.signum() as i32);
    } else if dy.abs() >= dx.abs() && dy.abs() >= dz.abs() {
      self.current = self.current.add_y(dy.signum() as i32);
    } else {
      self.current = self.current.add_z(dz.signum() as i32);
    }
    Some(ret)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
  }
}
