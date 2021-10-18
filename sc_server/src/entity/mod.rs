mod metadata;
mod ty;
mod version;

pub use metadata::Metadata;
pub use ty::{Data, Type};
pub use version::TypeConverter;

use parking_lot::Mutex;
use sc_common::math::FPos;

pub trait EntityData {
  /// The maximum health of this entity.
  fn max_health(&self) -> f32 {
    20.0
  }

  /// Returns true if the entity should despawn. Called whenever the entity's
  /// health changes, or when `check_despawn` is called.
  fn should_despawn(&self, health: f32) -> bool {
    health <= 0.0
  }
}

/// Default functionality for entities. Mostly used when an entity hasn't been
/// implemented.
struct DefaultEntity;
impl EntityData for DefaultEntity {}

pub struct Entity {
  /// The position of this entity. Must be valid for all entities.
  pos:    Mutex<FPos>,
  /// The type of this entity.
  ty:     Type,
  /// For some entities, such as projectiles, this field is ignored. To make the
  /// entity not disappear when it hits 0 health, overwrite the
  /// `should_despawn` function in `EntityData`.
  health: Mutex<f32>,
  data:   Mutex<Box<dyn EntityData + Send>>,
}

impl Entity {
  /// Reads this entity's position. This will always be up to date with the
  /// server's known position of this entity. Some clients may be behind this
  /// position (by up to 1/20 of a second).
  pub fn pos(&self) -> FPos {
    *self.pos.lock()
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

  /// Creates a new entity, with default functionality. They will take normal
  /// damage, and despawn if their health hits 0. If you want custom
  /// functionality of any kind, call [`new_custom`].
  pub fn new(ty: Type, pos: FPos) -> Self {
    Self::new_custom(ty, pos, DefaultEntity)
  }

  /// Creates a new entity, with the given functionality. This value will be
  /// store within the entity until it despawns.
  pub fn new_custom<D: EntityData + Send + 'static>(ty: Type, pos: FPos, data: D) -> Self {
    Entity {
      pos: Mutex::new(pos),
      ty,
      health: Mutex::new(data.max_health()),
      data: Mutex::new(Box::new(data)),
    }
  }
}
