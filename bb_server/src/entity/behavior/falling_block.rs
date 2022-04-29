use super::{Behavior, EntityData, EntityPos, ShouldDespawn};
use crate::world::World;
use bb_common::math::FPos;
use std::sync::Arc;

#[derive(Default)]
pub struct FallingBlock;

impl Behavior for FallingBlock {
  fn tick(&mut self, world: &Arc<World>, ent: &EntityData, p: &mut EntityPos) -> ShouldDespawn {
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

    if p.grounded {
      let ty = world
        .block_converter()
        .type_from_id(ent.data() as u32, bb_common::version::BlockVersion::latest());
      // This new position is at the center of the block, which will be the most
      // accurate for converting to a block position. If we don't add 0.5, then chains
      // of falling blocks can sometimes overlap.
      let _ = world.set_block((p.aabb.pos + FPos::new(0.0, 0.5, 0.0)).block(), ty);
      ShouldDespawn(true)
    } else {
      ShouldDespawn(false)
    }
  }
}
