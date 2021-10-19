use super::{FPos, Vec3};

pub struct AABB {
  pos:  FPos,
  // Never negative
  size: Vec3,
}

impl AABB {
  /// Creates a new axis-aligned bounding box. All the fields of `size` will be
  /// clamped to zero (negative sizes are now valid).
  pub fn new(pos: FPos, size: Vec3) -> Self {
    AABB { pos, size: Vec3::new(size.x.max(0.0), size.y.max(0.0), size.z.max(0.0)) }
  }

  /// Performs a collision with the other bounding box. This will move self such
  /// that self and other don't intersect. This should be called right after
  /// moving self. Returns true if self was moved.
  pub fn collide(&mut self, other: AABB) -> bool {
    if !self.is_colliding_with(other) {
      return false;
    }
    false
  }

  /// Returns true if self and other are intersecting. Being next to other
  /// (sides being equal) will return false.
  pub fn is_colliding_with(&self, other: AABB) -> bool {
    (self.min_x() > other.min_x() && self.min_x() < other.max_x())
      || (self.max_x() > other.min_x() && self.max_x() < other.max_x())
      || (self.min_y() > other.min_y() && self.min_y() < other.max_y())
      || (self.max_y() > other.min_y() && self.max_y() < other.max_y())
      || (self.min_z() > other.min_z() && self.min_z() < other.max_z())
      || (self.max_z() > other.min_z() && self.max_z() < other.max_z())
  }

  pub fn min_x(&self) -> f64 {
    self.pos.x()
  }
  pub fn min_y(&self) -> f64 {
    self.pos.y()
  }
  pub fn min_z(&self) -> f64 {
    self.pos.z()
  }
  pub fn max_x(&self) -> f64 {
    self.pos.x() + self.size.x
  }
  pub fn max_y(&self) -> f64 {
    self.pos.y() + self.size.y
  }
  pub fn max_z(&self) -> f64 {
    self.pos.z() + self.size.z
  }

  /// Returns the minimum position of this bounding box. Can be used to move the
  /// box around.
  pub fn pos_mut(&mut self) -> &mut FPos {
    &mut self.pos
  }
}
