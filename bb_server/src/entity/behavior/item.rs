use super::{Behavior, Entity, EntityPos, ShouldDespawn};

pub struct ItemBehavior {}

impl Default for ItemBehavior {
  fn default() -> Self { ItemBehavior {} }
}

impl Behavior for ItemBehavior {
  fn tick(&self, ent: &Entity, p: &mut EntityPos) -> ShouldDespawn {
    let _ = ent;
    let vel = p.vel;
    p.aabb.pos += vel;
    // This is for all projectiles. It is totally different on living entities.
    p.vel.x *= 0.99;
    p.vel.y *= 0.99;
    p.vel.z *= 0.99;
    if !p.grounded {
      p.vel.y -= 0.03;
    }

    for player in ent.world.read().players().iter().in_view(p.aabb.pos.block().chunk()) {
      if player.pos().dist_squared(p.aabb.pos) < 1.5_f64.powi(2) {
        player.lock_inventory().give(ent.metadata().get_item(8).into());
        return ShouldDespawn(true);
      }
    }
    ShouldDespawn(false)
  }
}
