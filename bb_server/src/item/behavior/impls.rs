use super::Behavior;
use crate::{
  block, entity,
  event::EventFlow::{self, *},
  player::{BlockClick, Click},
};
use bb_common::{
  math::FPos,
  util::{Chat, Face},
};

pub struct DebugStick;
impl Behavior for DebugStick {
  fn interact(&self, click: Click) -> EventFlow {
    if let Click::Block(click) = click {
      click.player.send_hotbar(Chat::new(click.block.ty.to_string()));
      Handled
    } else {
      Continue
    }
  }
  #[allow(clippy::collapsible_else_if)]
  fn break_block(&self, click: BlockClick) -> EventFlow {
    let mut block = click.block;
    let mut all_props: Vec<_> = block.ty.props().into_iter().collect();
    all_props.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let reverse = click.player.is_crouching();
    for (key, _) in all_props {
      let prop = block.ty.prop_at(&key).unwrap();
      let val = prop.id_of(&block.ty.prop(&key));
      if reverse {
        if val == 0 {
          let new_val = prop.from_id(prop.len() - 1);
          block.ty.set_prop(&key, new_val);
        } else {
          let new_val = prop.from_id(val - 1);
          block.ty.set_prop(&key, new_val);
          let _ = block.world.set_block(block.pos, block.ty);
          return Handled;
        }
      } else {
        if val == prop.len() - 1 {
          let new_val = prop.from_id(0);
          block.ty.set_prop(&key, new_val);
        } else {
          let new_val = prop.from_id(val + 1);
          block.ty.set_prop(&key, new_val);
          let _ = block.world.set_block(block.pos, block.ty);
          return Handled;
        }
      }
    }
    let _ = block.world.set_block(block.pos, block.ty);
    Handled
  }
}

pub struct Bucket(pub Option<block::Kind>);
impl Behavior for Bucket {
  fn interact(&self, click: Click) -> EventFlow {
    if let Some((pos, res)) = click.do_raycast(5.0, true) {
      let pos = (pos + res.axis / 2.0).block();
      let _ = match self.0 {
        Some(block) => click.player().world().set_kind(pos, block),
        None => click.player().world().set_kind(pos, block::Kind::Air),
      };
    }
    Handled
  }
}

pub struct Snowball;
impl Behavior for Snowball {
  fn interact(&self, click: Click) -> EventFlow {
    let eid = click
      .player()
      .world()
      .summon(entity::Type::Snowball, click.player().pos() + FPos::new(0.0, 1.0, 0.0));

    // If the entity doesn't exist, it already despawned, so we do nothing if
    // it isn't in the world.
    if let Some(ent) = click.player().world().entities().get(eid) {
      ent.set_vel(click.dir() * 1.5);
    }
    Continue
  }
}

pub struct Torch {
  pub normal: block::Kind,
  pub wall:   block::Kind,
}
impl Behavior for Torch {
  fn interact(&self, click: Click) -> EventFlow {
    if let Click::Block(mut click) = click {
      click.block.pos += click.face;
      if click.face == Face::Top {
        click.block.set(click.block.world.block_converter().ty(self.normal));
      } else if click.face != Face::Bottom {
        click.block.set(
          click.block.world.block_converter().ty(self.wall).with("facing", click.face.as_str()),
        );
      }
      Handled
    } else {
      Continue
    }
  }
}
