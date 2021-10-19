use super::{FPos, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct AABB {
  /// The position at the middle center of this AABB. This is the feet position
  /// of the entity, and is the value that should be sent in entity position
  /// packets.
  pub pos: FPos,
  /// Never negative
  size:    Vec3,
}

impl AABB {
  /// Creates a new axis-aligned bounding box. All the fields of `size` will be
  /// clamped to zero (negative sizes are now valid).
  pub fn new(pos: FPos, size: Vec3) -> Self {
    AABB { pos, size: Vec3::new(size.x.max(0.0), size.y.max(0.0), size.z.max(0.0)) }
  }

  /// Moves this box in the given direction, and make sure that it doesn't
  /// intersect with any of the given collision boxes.
  ///
  /// Returns true if this collided with anything.
  pub fn move_towards(&mut self, delta: Vec3, nearby: &[AABB]) -> bool {
    let mut collided = false;
    if !nearby.is_empty() {
      info!("collision time: we are {:?}, and are moving with delta {:?}", self, delta);
    }
    for &o in nearby {
      info!("handling collision with {:?}", o);
      let d = self.distance_from(o);
      info!("got distance {:?}", d);
      if d.x.abs() >= delta.x.abs() && d.y.abs() >= delta.y.abs() && d.z.abs() >= delta.z.abs() {
        continue;
      }
      info!("COLLISION TIME LADS");
      // If we get here, then moving self.pos by delta will cause us to
      // intersect `o`.
      collided = true;
    }

    if !collided {
      self.pos += delta;
    }
    collided
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

  /// Returns the distance from the other AABB in all axis. If the bounding
  /// boxes collide, then the value on that axis will be some negative value.
  /// The value should be ignored if it is less than zero.
  pub fn distance_from(&self, other: AABB) -> Vec3 {
    Vec3::new(
      if self.pos.x() > other.pos.x() {
        other.max_x() - self.min_x()
      } else {
        self.max_x() - other.min_x()
      },
      if self.pos.y() > other.pos.y() {
        other.max_y() - self.min_y()
      } else {
        self.max_y() - other.min_y()
      },
      if self.pos.z() > other.pos.z() {
        other.max_z() - self.min_z()
      } else {
        self.max_z() - other.min_z()
      },
    )
  }

  pub fn min_x(&self) -> f64 {
    self.pos.x() - self.size.x / 2.0
  }
  pub fn min_y(&self) -> f64 {
    self.pos.y()
  }
  pub fn min_z(&self) -> f64 {
    self.pos.z() - self.size.z / 2.0
  }
  pub fn max_x(&self) -> f64 {
    self.pos.x() + self.size.x / 2.0
  }
  pub fn max_y(&self) -> f64 {
    self.pos.y() + self.size.y
  }
  pub fn max_z(&self) -> f64 {
    self.pos.z() + self.size.z / 2.0
  }

  /// Returns the minimum position of this bounding box. Can be used to move the
  /// box around.
  pub fn pos_mut(&mut self) -> &mut FPos {
    &mut self.pos
  }
}
