use crate::{block, IntoFfi};
use bb_common::{math::FPos, transfer::MessageWriter};
use std::{error::Error, fmt, str::FromStr};

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  name: &'static str,
  id:   u32,
}

impl Data {
  /// Returns the particle's ID. This is the latest protocol ID.
  pub fn id(&self) -> u32 { self.id }
  /// Returns the name of this particle. This is something like `dust`. These
  /// don't have namespaces, because there aren't any namespaces for these on
  /// vanilla.
  ///
  /// TODO: Add namespaces.
  pub fn name(&self) -> &'static str { self.name }
}

#[derive(Debug)]
pub struct InvalidParticle(String);

impl fmt::Display for InvalidParticle {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid particle name: {}", self.0)
  }
}

impl Error for InvalidParticle {}

// Creates the type enum, and the generate_data function
include!(concat!(env!("OUT_DIR"), "/particle/ty.rs"));

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
  pub r: u8,
  pub g: u8,
  pub b: u8,
}

/// A cloud of particles.
pub struct Particle<'a> {
  /// The type of particle.
  pub ty:            Type<'a>,
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

impl IntoFfi for Particle<'_> {
  type Ffi = bb_ffi::CParticle;

  fn into_ffi(self) -> bb_ffi::CParticle {
    bb_ffi::CParticle {
      ty:            self.ty.into_ffi(),
      pos:           self.pos.into_ffi(),
      long_distance: self.long_distance.into_ffi(),
      offset:        self.offset.into_ffi(),
      count:         self.count,
      data:          self.data,
    }
  }
}
impl IntoFfi for Type<'_> {
  type Ffi = bb_ffi::CParticleType;

  fn into_ffi(self) -> bb_ffi::CParticleType {
    let mut data = vec![];
    let mut w = MessageWriter::new(&mut data);
    match self {
      Self::Block(block) => w.write_u32(block.id()).unwrap(),
      Self::BlockMarker(block) => w.write_u32(block.id()).unwrap(),
      Self::FallingDust(block) => w.write_u32(block.id()).unwrap(),
      Self::Dust(color, scale) => {
        w.write_u8(color.r).unwrap(); // r
        w.write_u8(color.g).unwrap(); // g
        w.write_u8(color.b).unwrap(); // b
        w.write_f32(scale).unwrap(); // scale
      }
      Self::DustColorTransition(from, to, scale) => {
        w.write_u8(from.r).unwrap(); // r
        w.write_u8(from.g).unwrap(); // g
        w.write_u8(from.b).unwrap(); // b
        w.write_f32(scale).unwrap(); // scale
        w.write_u8(to.r).unwrap(); // r
        w.write_u8(to.g).unwrap(); // g
        w.write_u8(to.b).unwrap(); // b
      }
      _ => {}
    }
    bb_ffi::CParticleType { ty: self.id(), data: data.into_ffi() }
  }
}
