use super::{Behavior, Entity, EntityPos, ShouldDespawn};

pub struct ItemBehavior {
  age: u32,
}

impl Default for ItemBehavior {
  fn default() -> Self { ItemBehavior { age: 0 } }
}

impl Behavior for ItemBehavior {
  fn tick(&mut self, ent: &Entity, p: &mut EntityPos) -> ShouldDespawn {
    let _ = ent;
    let vel = p.vel;
    p.aabb.pos += vel;
    // This is for items.
    p.vel.x *= 0.98;
    p.vel.y *= 0.98;
    p.vel.z *= 0.98;
    if !p.grounded {
      p.vel.y -= 0.04;
    }
    self.age += 1;

    if self.age >= 20 {
      for player in ent.world.read().players().iter().in_view(p.aabb.pos.block().chunk()) {
        if player.pos().dist_squared(p.aabb.pos) < 1.5_f64.powi(2) {
          player.lock_inventory().give(ent.metadata().get_item(8).into());
          return ShouldDespawn(true);
        }
      }
    }
    ShouldDespawn(false)
  }
}
