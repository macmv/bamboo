mod item;
mod snowball;

pub use item::ItemBehavior;
pub use snowball::SnowballBehavior;

use super::{EntityData, EntityPos, Type};

/// A wrapper type, to make it clear that `true` means an entity should be
/// removed.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ShouldDespawn(pub bool);

pub trait Behavior {
  /// The maximum health of this entity.
  fn max_health(&self) -> f32 { 20.0 }

  /// Returns true if the entity should despawn. Called whenever the entity's
  /// health changes, or when `check_despawn` is called.
  fn should_despawn(&self, health: f32) -> ShouldDespawn { ShouldDespawn(health <= 0.0) }

  /// Returns the amount of exp in this entity. For exp orbs, this is used when
  /// spawning them. For entities, this is how much exp will drop when they are
  /// killed.
  fn exp_count(&self) -> i32 { 1 }

  /// Any extra functionality needed. Called every tick, after movement and
  /// collision checks have been completed.
  fn tick(&mut self, ent: &EntityData, p: &mut EntityPos) -> ShouldDespawn {
    let _ = ent;
    let vel = p.vel;
    p.aabb.pos += vel;
    // 9.8 m/s ~= 0.5 m/tick. However, minecraft go brrr, and gravity is actually
    // 0.03 b/tick for projectiles, 0.04 b/tick for items, and 0.08 b/tick for
    // living entities.
    if !p.grounded {
      p.vel.y -= 0.08;
    }
    p.vel.y *= 0.98;

    // This is multiplied by the 'slipperiness' of the block the entity is standing
    // on.
    p.vel.x *= 0.91;
    p.vel.z *= 0.91;
    ShouldDespawn(false)
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
    Type::Item => Box::new(ItemBehavior::default()),
    _ => Box::new(DefaultBehavior::default()),
  }
}
