use sc_common::math::{FPos, Vec3};

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
    let mut result = None;
    let start = self.clone();
    let end = AABB::new(self.pos + delta, self.size);
    let mut time = 1.0;
    for &wall in nearby.iter().filter(|&&o| end.is_colliding_with(o)) {
      // Time to collide with the object, in each axis. We use this to find out which
      // axis will collide first.
      //
      // This two vectors store 6 numbers, each of which is a collision percentage. If
      // any of these collision percentages are within 0..1, then we have a potential
      // collision at that percentage.
      let t_min = Vec3::new(
        (wall.min_x() - start.max_x()) / delta.x,
        (wall.min_y() - start.max_y()) / delta.y,
        (wall.min_z() - start.max_z()) / delta.z,
      );
      let t_max = Vec3::new(
        (wall.max_x() - start.min_x()) / delta.x,
        (wall.max_y() - start.min_y()) / delta.y,
        (wall.max_z() - start.min_z()) / delta.z,
      );

      let mut axis = None;
      macro_rules! axis {
        ($time:expr, $axis:ident: $axis_val:expr, ($min_a:ident, $max_a:ident): $a:ident, ($min_b:ident, $max_b:ident): $b:ident) => {
          if $time.$axis == 0.0 {
            time = 0.0;
            axis = Some($axis_val);
          }
          if $time.$axis > 0.0 && $time.$axis < 1.0 && $time.$axis < time {
            let t = $time.$axis;
            let $min_a = start.$min_a() + delta.$a * t;
            let $max_a = start.$max_a() + delta.$a * t;
            let $min_b = start.$min_b() + delta.$b * t;
            let $max_b = start.$max_b() + delta.$b * t;
            if in_range(($min_a, $max_a), (wall.$min_a(), wall.$max_a()))
              && in_range(($min_b, $max_b), (wall.$min_b(), wall.$max_b()))
            {
              time = t;
              axis = Some($axis_val);
            }
          }
        };
      }

      axis!(t_min, x: Vec3::new(1.0, 0.0, 0.0), (min_y, max_y): y, (min_z, max_z): z);
      axis!(t_min, y: Vec3::new(0.0, 1.0, 0.0), (min_x, max_x): x, (min_z, max_z): z);
      axis!(t_min, z: Vec3::new(0.0, 0.0, 1.0), (min_x, max_x): x, (min_y, max_y): y);
      axis!(t_max, x: Vec3::new(1.0, 0.0, 0.0), (min_y, max_y): y, (min_z, max_z): z);
      axis!(t_max, y: Vec3::new(0.0, 1.0, 0.0), (min_x, max_x): x, (min_z, max_z): z);
      axis!(t_max, z: Vec3::new(0.0, 0.0, 1.0), (min_x, max_x): x, (min_y, max_y): y);

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

fn in_range(val: (f64, f64), range: (f64, f64)) -> bool {
  let (a, b) = val;
  let (min, max) = range;
  // this is a collision:
  // val   ->   ####
  // range -> ####
  // val   -> ####
  // range -> ####
  // val   -> ######
  // range -> ####
  // val   -> ##
  // range -> ####
  //
  // this is NOT a collision:
  // val   -> ####
  // range ->      ####
  // val   -> ####    (val.max == range.min)
  // range ->    ####
  // val   ->    #### (val.max == range.min)
  // range -> ####
  let a_inside = a > min && a < max;
  let b_inside = b > min && b < max;
  a_inside || b_inside || (a <= min && b >= max)
  // (a >= min && a <= max) || (b >= min && b <= max) || (a <= min && b >=
  // max)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn in_range_test() {
    assert!(in_range((1.0, 2.0), (0.0, 5.0)));
    assert!(in_range((0.0, 2.0), (0.0, 5.0)));
    assert!(in_range((2.0, 3.0), (0.0, 5.0)));
    assert!(in_range((0.0, 5.0), (0.0, 5.0)));
    assert!(!in_range((5.0, 6.0), (0.0, 5.0)));
    assert!(in_range((0.0, 5.0), (0.0, 1.0)));
    assert!(in_range((0.0, 5.0), (2.0, 3.0)));
  }

  #[test]
  fn collisions() {
    let mut b = AABB::new(FPos::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
    let walls = vec![
      AABB::new(FPos::new(5.0, 0.0, 0.0), Vec3::new(1.0, 10.0, 10.0)),
      AABB::new(FPos::new(0.0, 0.0, 10.0), Vec3::new(10.0, 10.0, 1.0)),
    ];

    assert!(b.move_towards(Vec3::new(1.0, 0.0, 0.0), &walls).is_none());
    assert_eq!(b.pos, FPos::new(1.0, 0.0, 0.0));

    assert!(b.move_towards(Vec3::new(1.0, 0.0, 0.0), &walls).is_none());
    assert_eq!(b.pos, FPos::new(2.0, 0.0, 0.0));

    assert!(b.move_towards(Vec3::new(1.0, 0.0, 0.0), &walls).is_none());
    assert_eq!(b.pos, FPos::new(3.0, 0.0, 0.0));

    let res = b.move_towards(Vec3::new(2.0, 0.0, 2.0), &walls).unwrap();
    assert_eq!(res.factor, 0.5);
    assert_eq!(b.pos, FPos::new(4.0, 0.0, 1.0));
  }
}
