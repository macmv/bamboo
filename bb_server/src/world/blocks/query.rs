use super::World;
use crate::{block, world::MultiChunk};
use bb_common::math::{ChunkPos, Pos, PosError};
use parking_lot::MutexGuard;
use std::collections::HashMap;

pub struct Query<'a> {
  world: &'a World,

  reads:  HashMap<ChunkPos, u32>,
  writes: HashMap<Pos, block::Type<'a>>,
}

/// We will try each query 3 times before failing
const CONTENTION_LIMIT: u32 = 3;

#[derive(Debug)]
pub enum QueryError {
  Contention,
  Pos(PosError),
}

impl From<PosError> for QueryError {
  fn from(e: PosError) -> Self { QueryError::Pos(e) }
}

impl World {
  /// Performs a query on the world. If this returns `Ok(R)`, then the entire
  /// query has succeeded.
  ///
  /// If a query succeeded, it means that all the blocks read during the query
  /// have stayed the same while all the writes were applied. For example, if
  /// you read that a block is air, then set it to stone, then setting it to
  /// stone will happen as a single transaction.
  ///
  /// Or, if you read that one block was grass, then set the block next to it to
  /// stone, then the grass block will not change until after this function
  /// returns.
  ///
  /// Additionally, this means that writing a block, and then reading from that
  /// same block will return the initial state of that block, before it was
  /// written. This also means that reading the same block will always return
  /// the same result.
  pub fn query<R>(&self, f: impl Fn(&mut Query) -> Result<R, QueryError>) -> Result<R, QueryError> {
    for _ in 0..CONTENTION_LIMIT {
      let mut query = Query::new(self);
      let res = match f(&mut query) {
        Ok(v) => v,
        Err(QueryError::Contention) => continue,
        Err(e) => return Err(e),
      };
      match query.apply() {
        Ok(()) => return Ok(res),
        Err(QueryError::Contention) => continue,
        Err(e) => return Err(e),
      }
    }
    Err(QueryError::Contention)
  }
}

/// Internal functions
impl<'a> Query<'a> {
  fn new(world: &'a World) -> Self {
    Query { world, reads: HashMap::new(), writes: HashMap::new() }
  }

  fn apply(self) -> Result<(), QueryError> { Ok(()) }
}

/// User-availible functions
impl<'a> Query<'a> {
  pub fn set_kind(&mut self, pos: Pos, kind: block::Kind) {
    self.writes.insert(pos, self.world.block_converter().ty(kind));
  }
  pub fn set_block(&mut self, pos: Pos, ty: block::Type<'a>) { self.writes.insert(pos, ty); }
  pub fn get_block(&mut self, pos: Pos) -> Result<block::TypeStore, QueryError> {
    self.read_chunk(pos.chunk(), |c| Ok(c.get_type(pos.chunk_rel())?.to_store()))
  }
  pub fn get_kind(&mut self, pos: Pos) -> Result<block::Kind, QueryError> {
    self.read_chunk(pos.chunk(), |c| Ok(c.get_kind(pos.chunk_rel())?))
  }

  fn read_chunk<R>(
    &mut self,
    pos: ChunkPos,
    f: impl FnOnce(MutexGuard<MultiChunk>) -> Result<R, QueryError>,
  ) -> Result<R, QueryError> {
    let current_version = self.reads.get(&pos).copied();
    self.world.chunk(pos, |c| {
      if let Some(current) = current_version {
        if c.version() > current {
          return Err(QueryError::Contention);
        }
      }
      self.reads.insert(pos, c.version());

      f(c)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn basics() {
    let world = World::new_test();
    world
      .query(|q| {
        let b = q.get_block(Pos::new(0, 0, 0))?;
        assert_eq!(b.kind(), block::Kind::Stone);

        q.set_kind(Pos::new(0, 0, 0), block::Kind::Air);

        let b = q.get_block(Pos::new(0, 0, 0))?;
        assert_eq!(b.kind(), block::Kind::Stone);

        Ok(())
      })
      .unwrap();
    // After the above transaction is applied, reads should give a new result
    world
      .query(|q| {
        let b = q.get_block(Pos::new(0, 0, 0))?;
        assert_eq!(b.kind(), block::Kind::Air);

        Ok(())
      })
      .unwrap();
  }
}
