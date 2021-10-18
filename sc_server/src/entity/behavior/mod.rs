mod snowball;

pub use snowball::SnowballBehavior;

use super::{Entity, Type};

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

  /// Any extra functionality needed. Called every tick, after movement and
  /// collision checks have been completed.
  fn tick(&self, ent: &Entity) {
    let _ = ent;
  }
}

/// Default functionality for entities. Mostly used when an entity hasn't been
/// implemented.
struct DefaultBehavior;
impl Behavior for DefaultBehavior {}

pub fn for_entity(ty: Type) -> Box<dyn Behavior + Send> {
  match ty {
    Type::Snowball => Box::new(SnowballBehavior),
    _ => Box::new(DefaultBehavior),
  }
}
