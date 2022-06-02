use crate::block;
use bb_common::{util::Buffer, version::ProtocolVersion};
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

impl Type {
  pub fn extra_data(&self, blocks: &block::TypeConverter, ver: ProtocolVersion) -> Vec<u8> {
    let mut data = vec![];
    let mut buf = Buffer::new(&mut data);
    match self {
      Self::Block(block) => buf.write_varint(blocks.to_old(block.id(), ver.block()) as i32),
      Self::BlockMarker(block) => {
        if ver > ProtocolVersion::V1_12_2 {
          buf.write_varint(blocks.to_old(block.id(), ver.block()) as i32)
        }
      }
      Self::FallingDust(block) => buf.write_varint(blocks.to_old(block.id(), ver.block()) as i32),
      Self::Dust(color, scale) => {
        if ver > ProtocolVersion::V1_12_2 {
          buf.write_f32(color.r as f32 / 255.0); // r
          buf.write_f32(color.g as f32 / 255.0); // g
          buf.write_f32(color.b as f32 / 255.0); // b
          buf.write_f32(*scale); // scale
        }
      }
      Self::DustColorTransition(from, to, scale) => {
        if ver > ProtocolVersion::V1_12_2 {
          buf.write_f32(from.r as f32 / 255.0); // r
          buf.write_f32(from.g as f32 / 255.0); // g
          buf.write_f32(from.b as f32 / 255.0); // b
          buf.write_f32(*scale); // scale
          buf.write_f32(to.r as f32 / 255.0); // r
          buf.write_f32(to.g as f32 / 255.0); // g
          buf.write_f32(to.b as f32 / 255.0); // b
        }
      }
      _ => {}
    }
    data
  }
}

#[cfg(test)]
mod tests {

  #[test]
  fn test_blocks() {
    // TODO: Re-enable when items are re-added
    /*
    let data = generate_items();

    // Sanity check some random blocks
    assert_eq!(data[0].block_to_place, block::Kind::Air);
    assert_eq!(data[1].block_to_place, block::Kind::Stone);
    assert_eq!(data[2].block_to_place, block::Kind::Granite);
    assert_eq!(data[182].block_to_place, block::Kind::DiamondBlock);
    assert_eq!(data[430].block_to_place, block::Kind::Observer);
    // Used to show debug output.
    // assert!(false);
    */
  }
}
