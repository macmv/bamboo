use super::World;
use crate::{entity, entity::Entity, player::Player};
use sc_common::{math::FPos, net::cb, util::UUID, version::ProtocolVersion};

impl World {
  pub fn summon(&self, ty: entity::Type, pos: FPos) -> i32 {
    let eid = self.eid();
    let ent = Entity::new(eid, ty, pos);

    for p in self.players().iter().in_view(pos.chunk()) {
      self.send_entity_spawn(p, &ent);
    }

    self.add_entity(eid, ent);
    eid
  }

  fn add_entity(&self, eid: i32, entity: Entity) {
    self.entities.write().insert(eid, entity);
  }

  fn send_entity_spawn(&self, player: &Player, ent: &Entity) {
    let p = ent.pos();
    dbg!(p);
    let id = ent.ty().to_u32();
    let ty_8;
    let ty_11;
    if player.ver() > ProtocolVersion::V1_11 {
      ty_11 = Some(self.entity_converter().to_old(id, player.ver().block()) as i32);
      ty_8 = None;
    } else {
      ty_8 = Some(self.entity_converter().to_old(id, player.ver().block()) as u8);
      ty_11 = None;
    }
    player.send(cb::Packet::SpawnEntityLiving {
      entity_id:              ent.eid(),
      entity_uuid_v1_9:       Some(UUID::from_u128(0)),
      type_v1_8:              ty_8,
      type_v1_11:             ty_11,
      x_v1_8:                 Some(p.fixed_x()),
      x_v1_9:                 Some(p.x()),
      y_v1_8:                 Some(p.fixed_y()),
      y_v1_9:                 Some(p.y()),
      z_v1_8:                 Some(p.fixed_z()),
      z_v1_9:                 Some(p.z()),
      yaw:                    0,
      pitch:                  0,
      head_pitch:             0,
      velocity_x:             0,
      velocity_y:             0,
      velocity_z:             0,
      metadata_removed_v1_15: Some(vec![0x7f]),
    });
  }
}
