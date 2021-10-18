mod metadata;
mod ty;
mod version;

pub use metadata::Metadata;
pub use ty::{Data, Type};
pub use version::TypeConverter;

use crate::world::World;
use parking_lot::{Mutex, RwLock};
use sc_common::math::{FPos, Vec3};
use std::sync::Arc;

pub trait Behavior {
  /// The maximum health of this entity.
  fn max_health(&self) -> f32 {
    20.0
  }

  /// Returns true if the entity should despawn. Called whenever the entity's
  /// health changes, or when `check_despawn` is called.
  fn should_despawn(&self, health: f32) -> bool {
    health <= 0.0
  }

  /// Returns the amount of exp in this entity. For exp orbs, this is used when
  /// spawning them. For entities, this is how much exp will drop when they are
  /// killed.
  fn exp_count(&self) -> i32 {
    1
  }
}

/// Default functionality for entities. Mostly used when an entity hasn't been
/// implemented.
struct DefaultBehavior;
impl Behavior for DefaultBehavior {}

pub struct Entity {
  /// The unique id for this entity. This is the key used to store entities in
  /// the World.
  eid:    i32,
  /// The position of this entity. Must be valid for all entities.
  pos:    Mutex<(FPos, Vec3)>,
  /// The type of this entity.
  ty:     Type,
  /// For some entities, such as projectiles, this field is ignored. To make the
  /// entity not disappear when it hits 0 health, overwrite the
  /// `should_despawn` function in `EntityData`.
  health: Mutex<f32>,
  /// The world this entity is in. Used whenever something changes, and nearby
  /// players need to be notified. This can change if the entity is teleported.
  world:  RwLock<Arc<World>>,
  data:   Mutex<Box<dyn Behavior + Send>>,
}

impl Entity {
  /// Creates a new entity, with default functionality. They will take normal
  /// damage, and despawn if their health hits 0. If you want custom
  /// functionality of any kind, call [`new_custom`].
  pub fn new(eid: i32, ty: Type, world: Arc<World>, pos: FPos) -> Self {
    Self::new_custom(eid, ty, pos, world, DefaultBehavior)
  }

  /// Creates a new entity, with the given functionality. This value will be
  /// store within the entity until it despawns.
  pub fn new_custom<D: Behavior + Send + 'static>(
    eid: i32,
    ty: Type,
    pos: FPos,
    world: Arc<World>,
    data: D,
  ) -> Self {
    Entity {
      eid,
      pos: Mutex::new((pos, Vec3::new(0.0, 0.0, 0.0))),
      ty,
      health: Mutex::new(data.max_health()),
      world: RwLock::new(world),
      data: Mutex::new(Box::new(data)),
    }
  }

  /// Reads this entity's position. This will always be up to date with the
  /// server's known position of this entity. Some clients may be behind this
  /// position (by up to 1/20 of a second).
  pub fn pos(&self) -> FPos {
    self.pos.lock().0
  }

  /// Returns the unique id for this entity.
  pub fn eid(&self) -> i32 {
    self.eid
  }

  /// Returns this entity's type. This can be used to send spawn packets to
  /// clients.
  pub fn ty(&self) -> Type {
    self.ty
  }

  /// Returns this entity's health.
  pub fn health(&self) -> f32 {
    *self.health.lock()
  }

  /// Returns true if this entity should despawn.
  pub fn should_despawn(&self) -> bool {
    self.data.lock().should_despawn(self.health())
  }

  /// Returns the amount of exp stored in this entity. This is just the amount
  /// for an exp orb, but it is also used to find out how much exp an entity
  /// will drop when killed.
  pub fn exp_count(&self) -> i32 {
    self.data.lock().exp_count()
  }

  /// Sets this entity's velocity. This will send velocity updates to nearby
  /// players, and will affect how the entity moves on the next tick.
  pub fn set_vel(&self, vel: Vec3) {
    self.pos.lock().1 = vel;
    self.world.read().send_entity_vel(self.pos().chunk(), self.eid, vel);
  }
}
