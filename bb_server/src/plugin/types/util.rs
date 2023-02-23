use super::Bamboo;
use crate::math::Vec3;
use bb_common::{
  math::{ChunkPos, FPos, Pos},
  util::{GameMode, UUID},
};
use bb_server_macros::define_ty;
use panda::runtime::{tree::Closure, LockedEnv};
use parking_lot::Mutex;
use std::sync::Arc;

/// A block position. This stores X, Y, and Z coordinates as ints.
///
/// If you need a player position, use `FPos` (for float position) instead.
#[define_ty]
impl PPos {
  info! {
    wrap: Pos,
    eq: true,

    panda: {
      path: "bamboo::util::Pos",
      map_key: true,
    },
  }

  /// Creates a new block position, with the given X, Y, and Z coordinates.
  pub fn new(x: i32, y: i32, z: i32) -> Self { PPos { inner: Pos::new(x, y, z) } }
  /// Returns the X position of this block.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.x() // returns 5
  /// ```
  pub fn x(&self) -> i32 { self.inner.x() }
  /// Returns the Y position of this block.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.y() // returns 6
  /// ```
  pub fn y(&self) -> i32 { self.inner.y() }
  /// Returns the Z position of this block.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.z() // returns 7
  /// ```
  pub fn z(&self) -> i32 { self.inner.z() }

  /// Returns a new position, with the X set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.with_x(10) // returns Pos::new(10, 6, 7)
  /// ```
  pub fn with_x(&self, new_x: i32) -> Self { self.inner.with_x(new_x).into() }
  /// Returns a new position, with the Y set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.with_y(10) // returns Pos::new(5, 10, 7)
  /// ```
  pub fn with_y(&self, new_y: i32) -> Self { self.inner.with_y(new_y).into() }
  /// Returns a new position, with the Z set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.with_z(10) // returns Pos::new(5, 6, 10)
  /// ```
  pub fn with_z(&self, new_z: i32) -> Self { self.inner.with_z(new_z).into() }

  /// Adds the given number in the X axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.add_x(10) // returns Pos::new(15, 6, 7)
  /// ```
  pub fn add_x(&self, offset: i32) -> Self { self.inner.add_x(offset).into() }
  /// Adds the given number in the Y axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.add_y(10) // returns Pos::new(5, 16, 7)
  /// ```
  pub fn add_y(&self, offset: i32) -> Self { self.inner.add_y(offset).into() }
  /// Adds the given number in the Z axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Pos::new(5, 6, 7)
  /// pos.add_z(10) // returns Pos::new(5, 6, 17)
  /// ```
  pub fn add_z(&self, offset: i32) -> Self { self.inner.add_z(offset).into() }
}

/// A chunk position. This stores X and Z coordinates.
///
/// If you need a block position, use `Pos` instead.
#[define_ty]
impl PChunkPos {
  info! {
    wrap: ChunkPos,
    eq: true,

    panda: {
      path: "bamboo::util::ChunkPos",
      map_key: true,
    },
  }

  /// Creates a new chunk position, with the given X and Z coordinates.
  pub fn new(x: i32, z: i32) -> Self { PChunkPos { inner: ChunkPos::new(x, z) } }
  /// Returns the X position of this chunk.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.x() // returns 5
  /// ```
  pub fn x(&self) -> i32 { self.inner.x() }
  /// Returns the Z position of this chunk.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.z() // returns 7
  /// ```
  pub fn z(&self) -> i32 { self.inner.z() }

  /// Returns a new position, with the X set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.with_x(10) // returns ChunkPos::new(10, 7)
  /// ```
  pub fn with_x(&self, new_x: i32) -> Self { self.inner.with_x(new_x).into() }
  /// Returns a new position, with the Z set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.with_z(10) // returns ChunkPos::new(5, 10)
  /// ```
  pub fn with_z(&self, new_z: i32) -> Self { self.inner.with_z(new_z).into() }

  /// Adds the given number in the X axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.add_x(10) // returns ChunkPos::new(15, 7)
  /// ```
  pub fn add_x(&self, offset: i32) -> Self { self.inner.add_x(offset).into() }
  /// Adds the given number in the Z axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = ChunkPos::new(5, 7)
  /// pos.add_z(10) // returns ChunkPos::new(5, 17)
  /// ```
  pub fn add_z(&self, offset: i32) -> Self { self.inner.add_z(offset).into() }
}

/// An entity position. This stores X, Y, and Z coordinates as floats.
///
/// If you need a block position, use `Pos` instead.
#[define_ty]
impl PFPos {
  info! {
    wrap: FPos,

    panda: {
      path: "bamboo::util::FPos",
    },
  }

  /// Creates a new floating point position, with the given X, Y, and Z
  /// coordinates.
  pub fn new(x: f64, y: f64, z: f64) -> Self { PFPos { inner: FPos::new(x, y, z) } }

  /// Returns the X position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.x() // returns 5.5
  /// ```
  pub fn x(&self) -> f64 { self.inner.x() }
  /// Returns the Y position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.y() // returns 6.0
  /// ```
  pub fn y(&self) -> f64 { self.inner.y() }
  /// Returns the Z position of this entity.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.z() // returns 7.2
  /// ```
  pub fn z(&self) -> f64 { self.inner.z() }

  /// Returns a new position, with the X set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.with_x(10.0) // returns FPos::new(10.0, 6.0, 7.2)
  /// ```
  pub fn with_x(&self, new_x: f64) -> Self { self.inner.with_x(new_x).into() }
  /// Returns a new position, with the Y set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.with_y(10.0) // returns FPos::new(5.5, 10.0, 7.2)
  /// ```
  pub fn with_y(&self, new_y: f64) -> Self { self.inner.with_y(new_y).into() }
  /// Returns a new position, with the Z set to the given value.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.with_z(10.0) // returns FPos::new(5.5, 6.0, 10.0)
  /// ```
  pub fn with_z(&self, new_z: f64) -> Self { self.inner.with_z(new_z).into() }

  /// Adds the given number in the X axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.add_x(100.0) // returns FPos::new(105.5, 6.0, 7.2)
  /// ```
  pub fn add_x(&self, offset: f64) -> Self { self.inner.add_x(offset).into() }
  /// Adds the given number in the Y axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.add_y(100.0) // returns FPos::new(5.5, 106.0, 7.2)
  /// ```
  pub fn add_y(&self, offset: f64) -> Self { self.inner.add_y(offset).into() }
  /// Adds the given number in the Z axis, and returns the new position.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.add_z(100.0) // returns FPos::new(5.5, 6.0, 107.2)
  /// ```
  pub fn add_z(&self, offset: f64) -> Self { self.inner.add_z(offset).into() }

  /// Returns the block that this position is in. This will round all 3 axis
  /// down.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.block() // returns Pos::new(5, 6, 7)
  /// ```
  pub fn block(&self) -> PPos { self.inner.block().into() }
}

/// A vector. This stores X, Y, and Z coordinates as floats.
///
/// If you need a position in the world, use `FPos` instead. This is used for
/// entity velocities, and raycasting math.
#[define_ty]
impl PVec3 {
  info! {
    wrap: Vec3,

    panda: {
      path: "bamboo::util::Vec3",
    },
  }

  /// Creates a new floating point position, with the given X, Y, and Z
  /// coordinates.
  pub fn new(x: f64, y: f64, z: f64) -> Self { PVec3 { inner: Vec3::new(x, y, z) } }

  /// Returns the X axis of this vector.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Vec3::new(5.5, 6.0, 7.2)
  /// pos.x // returns 5.5
  /// ```
  #[field]
  pub fn x(&self) -> f64 { self.inner.x }
  /// Returns the Y axis of this vector.
  ///
  /// # Example
  ///
  /// ```
  /// pos = FPos::new(5.5, 6.0, 7.2)
  /// pos.y() // returns 6.0
  /// ```
  #[field]
  pub fn y(&self) -> f64 { self.inner.y }
  /// Returns the Z axis of this vector.
  ///
  /// # Example
  ///
  /// ```
  /// pos = Vec3::new(5.5, 6.0, 7.2)
  /// pos.z() // returns 7.2
  /// ```
  #[field]
  pub fn z(&self) -> f64 { self.inner.z }
}

/// A UUID. This is used as a unique identifier for players and entities.
#[define_ty]
impl PUUID {
  info! {
    wrap: UUID,
    eq: true,

    panda: {
      path: "bamboo::util::UUID",
      map_key: true,
    },
  }

  /// Returns the UUID as a string, with dashes inserted.
  pub fn to_s(&self) -> String { self.inner.as_dashed_str() }
}

#[define_ty]
impl PGameMode {
  info! {
    wrap: GameMode,

    panda: {
      path: "bamboo::util::GameMode",
    },
  }
}

/// A duration. This is a number of ticks internally, and can be created from
/// a number of ticks, seconds, or minutes.
#[define_ty]
impl PDuration {
  info! {
    fields: {
      ticks: u32,
    },

    panda: {
      path: "bamboo::util::Duration",
    },
  }
  /// Returns a duration for the number of seconds specified.
  pub fn from_secs(secs: u32) -> Self { PDuration { ticks: secs * 20 } }
  /// Returns a duration for the number of minutes specified.
  pub fn from_minutes(minutes: u32) -> Self { PDuration { ticks: minutes * 20 * 60 } }
}

#[derive(Debug, Clone)]
pub struct CountdownData {
  active:    bool,
  /// Time left in seconds.
  time_left: u32,
  bamboo:    Bamboo,
  callback:  Closure,
}

impl CountdownData {
  fn update(&mut self, env: &mut LockedEnv) {
    match self.callback.call(env, vec![self.time_left.into()]) {
      Ok(_) => {}
      Err(e) => {
        // TODO: Log the error better
        // Stop the countdown, so that we don't emit a bunch of errors.
        error!("{e}");
        self.active = false;
      }
    }
  }
}

/// This is a timer, designed to be used for a minigame start countdown.
///
/// It can be easily set to decrease when more players have joined, and it can
/// also run a callback every time the timer changes.
#[define_ty]
impl PCountdown {
  info! {
    fields: {
      data: Arc<Mutex<CountdownData>>,
    },

    panda: {
      path: "bamboo::util::Countdown",
    },
  }

  /// Creates a new countdown, with the time set to the given number of seconds.
  ///
  /// The timer will call the given closure each second it decreases.
  ///
  /// The timer will be started. This means that in 1 second, the given closure
  /// will be called.
  pub fn new(bamboo: &Bamboo, time_left: u32, closure: Closure) -> Self {
    let c = PCountdown {
      data: Arc::new(Mutex::new(CountdownData {
        active: true,
        time_left,
        bamboo: bamboo.clone(),
        callback: closure,
      })),
    };
    let d = c.data.clone();
    c.data.lock().bamboo.after_loop_native(20, move |env| {
      let mut lock = d.lock();
      if lock.active && lock.time_left > 0 {
        lock.time_left -= 1;
        lock.update(env);
      }
    });
    c
  }
  /// On each timer update, the given closure will be called.
  pub fn on_change(&mut self, closure: Closure) { self.data.lock().callback = closure; }
  /// Sets the timer to have the given amount of time left.
  pub fn set(&mut self, time_left: u32) {
    let mut lock = self.data.lock();
    lock.time_left = time_left;

    let d = self.data.clone();
    lock.bamboo.after_native(0, move |env| d.lock().update(env));
  }
  /// If the given time is larger than the current time left, the time left will
  /// be set to the given value.
  ///
  /// This can be used when a player leaves, and you need to increase the amount
  /// of time until a game starts, but you don't want to decrease the time at
  /// all.
  ///
  /// # Example
  ///
  /// ```
  /// let count = Countdown::new(@bamboo, 20, |count| bamboo::info(count.to_s()))
  /// count.set_at_least(30) // time_left is now 30 seconds
  /// count.set_at_least(10) // time_left is still 30 seconds
  /// count.set_at_least(50) // time_left is now 50 seconds
  /// ```
  pub fn set_at_least(&mut self, time_left: u32) {
    let mut lock = self.data.lock();
    if time_left > lock.time_left {
      lock.time_left = time_left;

      let d = self.data.clone();
      lock.bamboo.after_native(0, move |env| d.lock().update(env));
    }
  }
  /// If the given time is smaller than the current time left, the time left
  /// will be set to the given value.
  ///
  /// This can be used when a player joines, and you need to decrease the amount
  /// of time until a game starts, but you don't want to increase the time at
  /// all.
  ///
  /// # Example
  ///
  /// ```
  /// let count = Countdown::new(@bamboo, 20, |count| bamboo::info(count.to_s()))
  /// count.set_at_most(30) // time_left is still 20 seconds
  /// count.set_at_most(10) // time_left is now 10 seconds
  /// count.set_at_most(50) // time_left is still 10 seconds
  /// ```
  pub fn set_at_most(&mut self, time_left: u32) {
    let mut lock = self.data.lock();
    if time_left < lock.time_left {
      lock.time_left = time_left;

      let d = self.data.clone();
      lock.bamboo.after_native(0, move |env| d.lock().update(env));
    }
  }
  /// Starts the countdown. This will do nothing if the countdown is already
  /// running.
  pub fn start(&mut self) { self.data.lock().active = true; }
  /// Stops the countdown. This will do nothing if the countdown isn't running.
  pub fn stop(&mut self) { self.data.lock().active = false; }
}
