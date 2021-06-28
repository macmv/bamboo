mod chunk_pos;
pub mod der;
mod fpos;
mod point_grid;
mod pos;
mod rng;
mod voronoi;

pub use chunk_pos::ChunkPos;
pub use fpos::{FPos, FPosError};
pub use point_grid::PointGrid;
pub use pos::{Pos, PosError};
pub use rng::WyhashRng;
pub use voronoi::Voronoi;

use sha1::{Digest, Sha1};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum BlockDirection {
  // Order matters here
  Down,
  Up,
  North,
  South,
  West,
  East,
}

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
        hex[i] += 1;
      }
    }
  }
  let out = format!("{:x}", hex).trim_start_matches('0').into();
  if negative {
    format!("-{}", out)
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
