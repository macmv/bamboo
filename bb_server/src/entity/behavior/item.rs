use super::{Behavior, EntityData, EntityPos, ShouldDespawn};
use crate::{item::Stack, world::World};
use bb_common::net::cb;
use std::sync::Arc;

#[derive(Default)]
pub struct ItemBehavior {
  age: u32,
}

impl Behavior for ItemBehavior {
  fn tick(&mut self, _: &Arc<World>, ent: &EntityData, p: &mut EntityPos) -> ShouldDespawn {
    let vel = p.vel;
    p.aabb.pos += vel;
    // This is for items.
    p.vel.x *= 0.98;
    p.vel.y *= 0.98;
    p.vel.z *= 0.98;
    if p.grounded {
      p.vel.x *= 0.6;
      p.vel.z *= 0.6;
    } else {
      p.vel.y -= 0.04;
    }
    self.age += 1;

    if self.age >= 10 {
      let pos = p.aabb.pos;
      let chunk = pos.block().chunk();
      for player in ent.world.read().players().iter().in_view(chunk) {
        if player.pos().dist_squared(p.aabb.pos) < 1.5_f64.powi(2) {
          let stack: Stack = ent.metadata().get_item(8).into();
          let amount = stack.amount();
          let remaining = player.lock_inventory().give(stack);

          if remaining == 0 {
            let collect =
              cb::Packet::CollectItem { item_eid: ent.eid(), player_eid: player.eid(), amount };
            // We want to include `player` in this loop, as they should also see the pickup
            // animation
            for other in player.world().players().iter().in_view(chunk) {
              other.send(collect.clone());
            }

            return ShouldDespawn(true);
          } else {
            // We still give the partial amount to the player (lock_inventory().give()), but
            // we don't show the pickup animation.
            let mut stack: Stack = ent.metadata().get_item(8).into();
            stack.set_amount(remaining);
            ent.metadata().set_item(8, stack.to_item());
          }
        }
      }
    }
    ShouldDespawn(false)
  }
}
