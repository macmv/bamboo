use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Clone, Copy)]
pub struct Instant(u64);
#[derive(Clone, Copy)]
pub struct Duration(u64);

impl Instant {
  pub fn now() -> Self { Instant(unsafe { bb_ffi::bb_time_since_start() }) }
  pub fn elapsed(&self) -> Duration { Instant::now() - *self }
  pub fn duration_since(&self, other: Instant) -> Duration { Duration(self.0 - other.0) }
  pub fn checked_add(&self, dur: Duration) -> Option<Instant> {
    Some(Instant(self.0.checked_add(dur.0)?))
  }
  pub fn checked_sub(&self, dur: Duration) -> Option<Instant> {
    Some(Instant(self.0.checked_sub(dur.0)?))
  }
}

impl Add<Duration> for Instant {
  type Output = Instant;

  fn add(self, other: Duration) -> Instant {
    self.checked_add(other).expect("overflow when adding duration to instant")
  }
}

impl AddAssign<Duration> for Instant {
  fn add_assign(&mut self, other: Duration) { *self = *self + other; }
}

impl Sub<Duration> for Instant {
  type Output = Instant;

  fn sub(self, other: Duration) -> Instant {
    self.checked_sub(other).expect("overflow when subtracting duration from instant")
  }
}

impl SubAssign<Duration> for Instant {
  fn sub_assign(&mut self, other: Duration) { *self = *self - other; }
}

impl Sub<Instant> for Instant {
  type Output = Duration;

  /// Returns the amount of time elapsed from another instant to this one,
  /// or zero duration if that instant is later than this one.
  ///
  /// # Panics
  ///
  /// Previous rust versions panicked when `other` was later than `self`.
  /// Currently this method saturates. Future versions may reintroduce the
  /// panic in some circumstances. See [Monotonicity].
  ///
  /// [Monotonicity]: Instant#monotonicity
  fn sub(self, other: Instant) -> Duration { self.duration_since(other) }
}

use std::fmt;

impl fmt::Debug for Duration {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    std::time::Duration::from_nanos(self.0).fmt(f)
  }
}
