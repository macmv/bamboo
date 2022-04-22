use super::{Behavior, EntityData, EntityPos, ShouldDespawn};

#[derive(Default)]
pub struct SnowballBehavior {
  _effect: Option<i32>,
}

impl Behavior for SnowballBehavior {
  fn tick(&mut self, _ent: &EntityData, p: &mut EntityPos) -> ShouldDespawn {
    let vel = p.vel;
    p.aabb.pos += vel;
    // This is for all projectiles. It is totally different on living entities.
    p.vel.x *= 0.99;
    p.vel.y *= 0.99;
    p.vel.z *= 0.99;
    if !p.grounded {
      p.vel.y -= 0.03;
    }
    ShouldDespawn(false)
  }
}
