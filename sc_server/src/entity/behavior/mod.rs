mod snowball;

pub use snowball::SnowballBehavior;

use super::{Entity, EntityPos, Type};

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
  fn tick(&self, ent: &Entity, p: &mut EntityPos) -> bool {
    let _ = ent;
    let vel = p.vel;
    p.pos += vel;
    // 9.8 m/s ~= 0.5 m/tick. However, minecraft go brrr, and gravity is actually
    // 0.03 b/tick for projectiles, and 0.08 b/tick for living entities.
    p.vel.y -= 0.08;
    p.vel.y *= 0.98;

    // This is multiplied by the 'sliperiness' of the block the entity is standing
    // on.
    p.vel.x *= 0.91;
    p.vel.z *= 0.91;
    false
  }
}

/// Default functionality for entities. Mostly used when an entity hasn't been
/// implemented.
#[derive(Default)]
struct DefaultBehavior;
impl Behavior for DefaultBehavior {}

pub fn for_entity(ty: Type) -> Box<dyn Behavior + Send> {
  match ty {
    Type::Snowball => Box::new(SnowballBehavior::default()),
    _ => Box::new(DefaultBehavior::default()),
  }
}
