use super::{super::chunk::MultiChunk, WorldGen};
use crate::block;
use sc_common::{
  math::{ChunkPos, Pos},
  version::BlockVersion,
};

const MAX_VER: i32 = 18;
const Y: i32 = 64;
const FILL: block::Kind = block::Kind::WhiteWool;
const EDGE: block::Kind = block::Kind::RedWool;

impl WorldGen {
  pub fn debug_world(&self, pos: ChunkPos, c: &mut MultiChunk) {
    let total_height = (MAX_VER - 8) * 3 + 2;

    let fill = if pos.x() >= 0 && pos.z() >= 0 && pos.z() % 3 != 2 && pos.z() < total_height {
      block::Kind::Air
    } else {
      FILL
    };
    c.fill_kind(Pos::new(0, Y - 1, 0), Pos::new(15, Y - 1, 15), fill).unwrap();

    // First, draw the ground
    if pos.z() < 0 || pos.x() < -1 {
      self.debug_numbers(pos, c);
      return;
    }
    if pos.x() == -1 {
      self.debug_numbers(pos, c);
    }

    if pos.x() < -1 || pos.z() < -1 || pos.z() > total_height {
      return;
    }

    let is_left = pos.x() == -1;
    let is_top = (pos.z() == -1 || pos.z() % 3 == 2) && pos.z() < total_height;
    let is_bottom = pos.z() % 3 == 2;
    if is_left {
      if is_top {
        // Top left corner
        c.set_kind(Pos::new(15, Y - 1, 15), EDGE).unwrap();
      }
      if is_bottom {
        // Bottom left corner
        c.set_kind(Pos::new(15, Y - 1, 0), EDGE).unwrap();
      }
      if !is_top && !is_bottom {
        // Left edge
        c.fill_kind(Pos::new(15, Y - 1, 0), Pos::new(15, Y - 1, 15), EDGE).unwrap();
      }
    } else {
      if is_top {
        c.fill_kind(Pos::new(0, Y - 1, 15), Pos::new(15, Y - 1, 15), EDGE).unwrap();
      }
      if is_bottom {
        c.fill_kind(Pos::new(0, Y - 1, 0), Pos::new(15, Y - 1, 0), EDGE).unwrap();
      }
    }

    // Then we place the actual blocks

    if pos.z() < 0 || pos.x() < 0 || pos.z() % 3 == 2 {
      return;
    }
    let mut maj = pos.z() / 3 + 8;
    if maj > MAX_VER {
      return;
    }
    if maj == 13 {
      return;
    }
    if maj > 13 {
      maj -= 1;
    }
    let ver = BlockVersion::from_index(maj as u32 - 8);
    for x in pos.block_x()..pos.block_x() + 16 {
      if x % 2 != 0 {
        continue;
      }
      let id = x / 2;
      for z in 0..16 {
        if z % 2 != 0 {
          continue;
        }
        let state = (z + pos.block_z() % 24) / 2;
        let ty = c.type_converter().type_from_id(id as u32 * 16 + state as u32, ver);
        c.set_type(Pos::new(x % 16, Y, z), ty).unwrap();
      }
    }
  }

  fn debug_numbers(&self, pos: ChunkPos, c: &mut MultiChunk) {
    // We want a design like so:
    //
    // ```
    //                      #
    //                      #
    //                      #
    //                      #
    //                      #
    //
    // ### ### # # ### ### ###
    // # #   # # # #   # # # #
    // # # ### ### ### ### # #
    // # # #     # # # # # # #
    // ### ###   # ### ### ###
    //
    //  #   #   #   #   #   #
    //  # # # # # # # # # # # #
    // ##########################
    // # ~ blocks here ~
    // #########################
    // ```

    if pos.x() >= -5 && pos.x() <= -1 && pos.z() >= 0 && pos.z() % 3 == 0 {
      const X: i32 = -20;
      let ver = (pos.z() / 3 + 8) as u8;
      if ver <= MAX_VER as u8 {
        self.place_digit(pos, c, 1, X, pos.block_z() + 10);
        let dot = Pos::new(X + 3, Y - 1, pos.block_z() + 14);
        if dot.chunk() == pos {
          c.set_kind(dot.chunk_rel(), EDGE).unwrap();
        }
        if ver < 10 {
          self.place_digit(pos, c, ver, X + 5, pos.block_z() + 10);
        } else {
          self.place_digit(pos, c, ver / 10, X + 5, pos.block_z() + 10);
          self.place_digit(pos, c, ver % 10, X + 9, pos.block_z() + 10);
        }
      }
    }

    if pos.x() < -1 {
      return;
    }
    if pos.z() == -1 {
      if pos.x() == -1 {
        // Corner
        c.set_kind(Pos::new(15, Y - 1, 15), EDGE).unwrap();
      }
      if pos.x() >= 0 {
        c.fill_kind(Pos::new(0, Y - 1, 15), Pos::new(15, Y - 1, 15), EDGE).unwrap();
        for x in 0..16 {
          if x % 4 == 0 {
            c.fill_kind(Pos::new(x, Y - 1, 13), Pos::new(x, Y - 1, 14), EDGE).unwrap();
          } else if x % 4 == 2 {
            c.set_kind(Pos::new(x, Y - 1, 14), EDGE).unwrap();
          }
        }
      }
    }
    for num in 0..5 {
      let id = (pos.x() * 4 + num) * 2;
      if id < 0 {
        continue;
      }
      let left = pos.block_x() + num * 4 - 1;
      self.place_number(pos, c, id, left);
    }
  }

  fn place_number(&self, pos: ChunkPos, c: &mut MultiChunk, mut id: i32, left: i32) {
    if id == 0 {
      self.place_digit(pos, c, 0, left, -9);
      return;
    }
    let mut z = -9;
    while id != 0 {
      self.place_digit(pos, c, (id % 10) as u8, left, z);
      z -= 6;
      id /= 10;
    }
  }

  fn place_digit(&self, chunk_pos: ChunkPos, c: &mut MultiChunk, digit: u8, left: i32, top: i32) {
    let num: [&str; 5] = [
      ["###", "# #", "# #", "# #", "###"], // 0
      [" # ", " # ", " # ", " # ", " # "], // 1
      ["###", "  #", "###", "#  ", "###"], // 2
      ["###", "  #", "###", "  #", "###"], // 3
      ["# #", "# #", "###", "  #", "  #"], // 4
      ["###", "#  ", "###", "  #", "###"], // 5
      ["###", "#  ", "###", "# #", "###"], // 6
      ["###", "  #", "  #", "  #", "  #"], // 7
      ["###", "# #", "###", "# #", "###"], // 8
      ["###", "# #", "###", "  #", "###"], // 9
    ][digit as usize];
    for x in 0..3 {
      for z in 0..5 {
        let ch = &num[z as usize][x as usize..x as usize + 1];
        let pos = Pos::new(left + x, Y - 1, top + z);
        if pos.chunk_x() == chunk_pos.x() && pos.chunk_z() == chunk_pos.z() && ch == "#" {
          c.set_kind(pos.chunk_rel(), EDGE).unwrap();
        }
      }
    }
  }
}
