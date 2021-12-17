use super::{
  super::{math::PointGrid, BiomeGen},
  WorldGen,
};
use crate::{block, world::chunk::MultiChunk};
use sc_common::math::{ChunkPos, Pos};

pub struct Gen {
  id:    usize,
  trees: PointGrid,
}

impl Gen {
  pub fn place_tree(
    &self,
    world: &WorldGen,
    c: &mut MultiChunk,
    tree_pos: Pos,
    chunk_pos: ChunkPos,
  ) {
    // Iterate through each of the columns of the tree
    for offset in Pos::new(-2, 0, -2).to(Pos::new(2, 0, 2)) {
      // We only want to place things if this column is in the right chunk
      let p = tree_pos + offset;
      let leaf_start;
      if world.chance(tree_pos, 0.50) {
        leaf_start = 3
      } else {
        leaf_start = 2
      }
      if p.chunk() == chunk_pos {
        let mut rel = p.chunk_rel();
        if p == tree_pos {
          c.fill_kind(rel, rel.add_y(4), block::Kind::OakLog).unwrap();
          c.fill_kind(rel.add_y(5), rel.add_y(leaf_start + 3), block::Kind::OakLeaves).unwrap();
        } else {
          rel += Pos::new(0, leaf_start, 0);
          // If this is false, then it is the outside corner, where we don't want leaves
          if (offset.x() > -2 && offset.x() < 2) || (offset.z() > -2 && offset.z() < 2) {
            // If this is true, we are on the outside ring, where the leaves should be lower
            // If this is false, we are in the middle 9 columns
            if offset.x() == -2 || offset.x() == 2 || offset.z() == -2 || offset.z() == 2 {
              c.fill_kind(rel, rel.add_y(2), block::Kind::OakLeaves).unwrap();
            } else {
              // If this is true, we are in one of the outside corners of the middle ring
              // If this is false, we are in the middle cross of 5 columns
              if (offset.x() == -1 || offset.x() == 1) && (offset.z() == -1 || offset.z() == 1) {
                // We want a 25% change of placing a leaf in this corner
                if world.chance(p, 0.25) {
                  c.fill_kind(rel, rel.add_y(3), block::Kind::OakLeaves).unwrap();
                } else {
                  c.fill_kind(rel, rel.add_y(2), block::Kind::OakLeaves).unwrap();
                }
              } else {
                c.fill_kind(rel, rel.add_y(3), block::Kind::OakLeaves).unwrap();
              }
            }
          }
        }
      }
    }
  }
}

impl BiomeGen for Gen {
  fn new(id: usize) -> Gen { Gen { id, trees: PointGrid::new(12345, 16, 5) } }
  fn id(&self) -> usize { self.id }
  fn decorate(&self, world: &WorldGen, chunk_pos: ChunkPos, c: &mut MultiChunk) {
    // Iterate through a 2 block radius outside this chunk
    for p in
      (chunk_pos.block() - Pos::new(2, 0, 2)).to(chunk_pos.block() + Pos::new(15 + 2, 0, 15 + 2))
    {
      if world.is_biome(self, p) && self.trees.contains(p.into()) {
        let height = self.height_at(world, p);
        self.place_tree(world, c, p.with_y(height + 1), chunk_pos);
      }
    }
  }
}
