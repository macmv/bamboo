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

#[derive(Debug, Clone)]
pub struct CollisionResult {
  /// A unit vector in one direction, representing which direction the collision
  /// was in. This can be negative, so there are 6 possible values here.
  pub axis:   Vec3,
  /// How much of the delta was completed. If this is 1, then we didn't collide
  /// with anything. If this was 0, then we didn't move at all.
  pub factor: f64,
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
  pub fn move_towards(&mut self, delta: Vec3, nearby: &[AABB]) -> Option<CollisionResult> {
    fn time_factor(val: f64, start: f64, end: f64) -> f64 { (val - start) / (end - start) }

    let mut result = None;
    let after_move = AABB::new(self.pos + delta, self.size);
    let mut time = 1.0;
    for &o in nearby.iter().filter(|&&o| after_move.is_colliding_with(o)) {
      let d = self.distance_from(o);
      // Time to collide with the object, in each axis. We use this to find out which
      // axis will collide first.
      let t = Vec3::new(d.x / delta.x, d.y / delta.y, d.z / delta.z);

      let mut axis = None;
      if t.x <= t.y && t.x <= t.z {
        // Collided on the X axis
        let pos_x;
        if delta.x > 0.0 {
          pos_x = o.min_x() - self.size.x / 2.0;
        } else {
          pos_x = o.max_x() + self.size.x / 2.0;
        }
        let fac = time_factor(pos_x, self.pos.x, after_move.pos.x);
        if fac < time {
          time = fac;
          axis = Some(Vec3::new(delta.x.signum(), 0.0, 0.0));
        }
      } else if t.y <= t.x && t.y <= t.z {
        // Collided on the Y axis
        let pos_y;
        // Y is different, because self.pos is the bottom, not middle.
        if delta.y > 0.0 {
          pos_y = o.min_y() - self.size.y;
        } else {
          pos_y = o.max_y();
        }
        let fac = time_factor(pos_y, self.pos.y, after_move.pos.y);
        if fac < time {
          time = fac;
          axis = Some(Vec3::new(0.0, delta.y.signum(), 0.0));
        }
      } else {
        // Collided on the Z axis
        let pos_z;
        if delta.z > 0.0 {
          pos_z = o.min_z() - self.size.z / 2.0;
        } else {
          pos_z = o.max_z() + self.size.z / 2.0;
        }
        let fac = time_factor(pos_z, self.pos.z, after_move.pos.z);
        if fac < time {
          time = fac;
          axis = Some(Vec3::new(0.0, 0.0, delta.z.signum()));
        }
      }
      if let Some(axis) = axis {
        result = Some(CollisionResult { axis, factor: time })
      }
    }

    if result.is_some() {
      self.pos += delta * time;
    } else {
      self.pos += delta;
    }
    result
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

  pub fn min_x(&self) -> f64 { self.pos.x() - self.size.x / 2.0 }
  pub fn min_y(&self) -> f64 { self.pos.y() }
  pub fn min_z(&self) -> f64 { self.pos.z() - self.size.z / 2.0 }
  pub fn max_x(&self) -> f64 { self.pos.x() + self.size.x / 2.0 }
  pub fn max_y(&self) -> f64 { self.pos.y() + self.size.y }
  pub fn max_z(&self) -> f64 { self.pos.z() + self.size.z / 2.0 }

  /// Returns the minimum position of this bounding box. Can be used to move the
  /// box around.
  pub fn pos_mut(&mut self) -> &mut FPos { &mut self.pos }
}
