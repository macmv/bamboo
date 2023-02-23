use super::{
  block::{PBlockKind, PBlockType},
  item::PStack,
  util::{PFPos, PPos},
};
use crate::{entity, world::World};
use bb_common::{math::Pos, metadata::Metadata, net::cb::SoundCategory};
use bb_server_macros::define_ty;
use panda::{parse::token::Span, runtime::RuntimeError};
use std::{fmt, sync::Arc};

pub mod gen;

impl fmt::Debug for PWorld {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.debug_struct("PWorld").finish() }
}

impl PWorld {
  pub fn check_pos(&self, pos: Pos) -> Result<Pos, RuntimeError> {
    self.inner.check_pos(pos).map_err(|p| {
      RuntimeError::custom(format!("invalid position {}: {}", p.pos, p.msg), Span::call_site())
    })
  }
}

/// A Minecraft world. This stores all of the information about blocks,
/// entities, and players in this world.
#[define_ty]
impl PWorld {
  info! {
    debug: false,
    wrap: Arc<World>,

    panda: {
      path: "bamboo::world::World",
    },
  }
  /// Sets a single block in the world. This will return an error if the block
  /// is outside of the world.
  ///
  /// If you need to set multiple blocks at all, you should always use
  /// `fill_rect` instead. It is faster in every situation except for single
  /// blocks (where it is the same speed).
  ///
  /// This function will do everything you want in a block place. It will update
  /// the blocks stored in the world, and send block updates to all clients in
  /// render distance.
  pub fn set_block(&self, pos: &PPos, kind: &PBlockKind) -> Result<(), RuntimeError> {
    self.check_pos(pos.inner)?;
    self.inner.set_kind(pos.inner, kind.inner).unwrap();
    Ok(())
  }
  /// Tries to set a block at the given position. This will return `false` if
  /// the block is outside the world, and `true` if the block is in the world.
  pub fn try_set_block(&self, pos: &PPos, kind: &PBlockKind) -> bool {
    self.inner.set_kind(pos.inner, kind.inner).is_ok()
  }

  /// Fills a rectangle of blocks in the world. This will return an error if the
  /// min or max are outside of the world.
  ///
  /// This function will do everything you want when filling blocks.. It will
  /// update the blocks stored in the world, and send block updates to all
  /// clients in render distance.
  pub fn fill_rect(&self, min: &PPos, max: &PPos, kind: &PBlockKind) -> Result<(), RuntimeError> {
    self.check_pos(min.inner)?;
    self.check_pos(max.inner)?;
    self.inner.fill_rect_kind(min.inner, max.inner, kind.inner).unwrap();
    Ok(())
  }

  /// Returns the block type at the given position.
  ///
  /// This will return an error if the position is outside the world.
  pub fn get_block(&self, pos: &PPos) -> Result<PBlockType, RuntimeError> {
    self.check_pos(pos.inner)?;
    Ok(self.inner.get_block(pos.inner).unwrap().into())
  }

  /// Summons a dropped item at the given position.
  pub fn summon_item(&self, pos: &PFPos, stack: &PStack) {
    let mut meta = Metadata::new();
    meta.set_item(8, stack.inner.to_item());
    self.inner.summon_meta(entity::Type::Item, pos.inner, meta);
  }

  /// Plays the given sound at the given positions. All nearby players will be
  /// able to hear it.
  pub fn play_sound(
    &self,
    sound: &str,
    category: &str,
    pos: &PFPos,
    volume: f32,
    pitch: f32,
  ) -> Result<(), RuntimeError> {
    self.inner.play_sound(
      sound.into(),
      match category {
        "master" => SoundCategory::Master,
        "music" => SoundCategory::Music,
        "record" => SoundCategory::Records,
        "weather" => SoundCategory::Weather,
        "block" => SoundCategory::Blocks,
        "hostile" => SoundCategory::Hostile,
        "neutral" => SoundCategory::Neutral,
        "player" => SoundCategory::Players,
        "ambient" => SoundCategory::Ambient,
        "voice" => SoundCategory::Voice,
        _ => {
          return Err(RuntimeError::custom(
            format!("invalid sound category: {category}"),
            Span::call_site(),
          ))
        }
      },
      pos.inner,
      volume,
      pitch,
    );
    Ok(())
  }

  /// Saves the world to disk. If saving is disabled, the world will not be
  /// saved.
  pub fn save(&self) { self.inner.save(); }
}
