mod chunk_pos;
#[cfg(feature = "host")]
pub mod der;
mod fast;
mod fpos;
mod pos;
mod rel_pos;
#[cfg(feature = "host")]
mod rng;

pub use fast::{FastMath, EPSILON};
pub use fpos::{FPos, FPosError};
#[cfg(feature = "host")]
pub use rng::WyhashRng;

#[cfg(feature = "host")]
pub use rand_core::RngCore;

use sha1::{Digest, Sha1};
use std::hash::BuildHasher;
use wyhash::WyHash;

pub use chunk_pos::ChunkPos;
pub use pos::{Pos, PosError, PosIter};
pub use rel_pos::{RelPos, SectionRelPos};
// pub use bb_generated::{ChunkPos, Pos, PosError, PosIter};

/// The minecraft hex digest. This is slightly different from a normal hex
/// digest; see [the wiki](https://wiki.vg/Protocol_Encryption) for more information.
pub fn hexdigest(hash: Sha1) -> String {
  let mut hex = hash.finalize();

  let negative = (hex[0] & 0x80) == 0x80;

  if negative {
    let mut carry = true;
    for i in (0..hex.len()).rev() {
      hex[i] = !hex[i];
      if carry {
        carry = hex[i] == 0xff;
        // Unclear if wrapping add is expected here, but I cannot find an example of
        // this online. Assuming that this was implemented in the same way in java, then
        // this should be a wrapping add.
        hex[i] = hex[i].wrapping_add(1);
      }
    }
  }
  let out = format!("{hex:x}").trim_start_matches('0').into();
  if negative {
    format!("-{out}")
  } else {
    out
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sha1() {
    let mut hash = Sha1::new();
    hash.update(b"Notch");
    assert_eq!(hexdigest(hash), "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48");

    let mut hash = Sha1::new();
    hash.update(b"jeb_");
    assert_eq!(hexdigest(hash), "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1");

    let mut hash = Sha1::new();
    hash.update(b"simon");
    assert_eq!(hexdigest(hash), "88e16a1019277b15d58faf0541e11910eb756f6");
  }

  #[test]
  fn test_overflow() {
    for c1 in ' '..'~' {
      for c2 in ' '..'~' {
        let mut hash = Sha1::new();
        hash.update([c1 as u8, c2 as u8]);
        hexdigest(hash);
      }
    }
  }
}

#[derive(Debug, Default, Clone)]
pub struct WyHashBuilder;

impl BuildHasher for WyHashBuilder {
  type Hasher = WyHash;

  fn build_hasher(&self) -> Self::Hasher {
    // Random number. This hashser should only be used when we don't care about DOS
    // situations.
    WyHash::with_seed(0xe6cc56f1f7550d95)
  }
}
