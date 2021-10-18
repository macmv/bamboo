use super::World;
use crate::{entity, entity::Entity, player::Player};
use sc_common::{math::FPos, net::cb, util::UUID};

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
    let id = ent.ty().id();
    let old_id = self.entity_converter().to_old(id, player.ver().block());
    info!("modern id: {}", id);
    info!("old id: {:?}", old_id);
    if ent.ty() == entity::Type::ExperienceOrb {
      player.send(cb::Packet::SpawnEntityExperienceOrb {
        entity_id: ent.eid(),
        x_v1_8:    Some(p.fixed_x()),
        x_v1_9:    Some(p.x()),
        y_v1_8:    Some(p.fixed_y()),
        y_v1_9:    Some(p.y()),
        z_v1_8:    Some(p.fixed_z()),
        z_v1_9:    Some(p.z()),
        count:     ent.exp_count() as i16,
      });
    } else if ent.ty() == entity::Type::Painting {
      player.send(cb::Packet::SpawnEntityPainting {
        entity_id:        ent.eid(),
        entity_uuid_v1_9: Some(UUID::from_u128(0)),
        title_v1_8:       Some("hello".into()),
        title_v1_13:      Some(0),
        location:         p.block(),
        // TODO: Infer this from the entity yaw
        direction:        0,
      });
    } else if ent.ty().is_living() {
      player.send(cb::Packet::SpawnEntityLiving {
        entity_id:              ent.eid(),
        entity_uuid_v1_9:       Some(UUID::from_u128(0)),
        type_v1_8:              Some(old_id as u8),
        type_v1_11:             Some(old_id as i32),
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
    } else {
      // Data is some data specific to that entity. If it is non-zero, then velocity
      // is present.
      let data: i32 = 0;
      player.send(cb::Packet::SpawnEntity {
        entity_id:        ent.eid(),
        object_uuid_v1_9: Some(UUID::from_u128(0)),
        type_v1_8:        Some(old_id as i8),
        type_v1_14:       Some(old_id as i32),
        x_v1_8:           Some(p.fixed_x()),
        x_v1_9:           Some(p.x()),
        y_v1_8:           Some(p.fixed_y()),
        y_v1_9:           Some(p.y()),
        z_v1_8:           Some(p.fixed_z()),
        z_v1_9:           Some(p.z()),
        pitch:            0,
        yaw:              0,
        object_data_v1_8: Some(data.to_le_bytes().to_vec()),
        object_data_v1_9: Some(data),
        velocity_x_v1_9:  None,
        velocity_y_v1_9:  None,
        velocity_z_v1_9:  None,
      });
    }
  }
}
