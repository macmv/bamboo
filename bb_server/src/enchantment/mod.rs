mod ty;
mod version;

pub use ty::{Data, Type};
pub use version::TypeConverter;

use std::num::NonZeroU8;

/// An enchantment.
#[derive(Clone, Debug)]
pub struct Particle {
  /// The type of particle.
  pub id:    Type,
  /// The level of enchantment.
  pub level: NonZeroU8,
}
