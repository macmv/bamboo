mod ty;
mod version;

#[cfg(feature = "wasm_plugins")]
mod ffi;

pub use ty::{Color, Data, Type};
pub use version::TypeConverter;

use crate::block;
use bb_common::{math::FPos, net::cb, version::ProtocolVersion};

/// A cloud of particles.
#[derive(Clone, Debug)]
pub struct Particle {
  /// The type of particle.
  pub ty:            Type,
  /// The center of this cloud of particles.
  pub pos:           FPos,
  /// If set, the particle will be shown to clients up to 65,000 blocks away. If
  /// not set, the particle will only render up to 256 blocks away.
  pub long_distance: bool,
  /// The random offset for this particle cloud. This is multiplied by a random
  /// number from 0 to 1, and then added to `pos` (all on the client).
  pub offset:        FPos,
  /// The number of particles in this cloud.
  pub count:         u32,
  /// The data for this particle. This is typically the speed of the particle,
  /// but sometimes is used for other attributes entirely.
  pub data:          f32,
}

impl Particle {
  pub fn to_packet(
    &self,
    blocks: &block::TypeConverter,
    ver: ProtocolVersion,
  ) -> cb::packet::Particle {
    cb::packet::Particle {
      id:         self.ty.id() as i32,
      long:       self.long_distance,
      pos:        self.pos,
      offset:     self.offset,
      data_float: self.data,
      count:      self.count as i32,
      data:       self.ty.extra_data(blocks, ver),
    }
  }
}
