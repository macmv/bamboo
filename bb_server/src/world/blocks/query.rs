use super::World;
use crate::{block, world::MultiChunk};
use bb_common::math::{ChunkPos, Pos, PosError, RelPos};
use parking_lot::MutexGuard;
use std::collections::HashMap;

pub struct Query<'a> {
  world: &'a World,

  reads:  HashMap<ChunkPos, u32>,
  writes: HashMap<ChunkPos, HashMap<RelPos, block::Type<'a>>>,
}

/// We will try each query 3 times before failing
const CONTENTION_LIMIT: u32 = 3;

#[derive(Debug, PartialEq)]
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

  fn apply(self) -> Result<(), QueryError> {
    // Validate that none of the read-only chunks have changed.
    for (pos, version) in &self.reads {
      if self.writes.contains_key(pos) {
        continue;
      }
      self.world.chunk(*pos, |c| {
        if c.version() != *version {
          return Err(QueryError::Contention);
        }
        Ok(())
      })?;
    }
    // None of the read-only chunks have moved, so we go write everything. If any of
    // the write chunks are also read chunks, we make sure they haven't changed.
    for (pos, writes) in self.writes {
      self.world.chunk(pos, |mut c| {
        if let Some(ver) = self.reads.get(&pos) {
          if *ver != c.version() {
            return Err(QueryError::Contention);
          }
        }
        for (pos, ty) in writes {
          c.set_type(pos, ty)?;
        }
        Ok(())
      })?;
    }
    Ok(())
  }
}

/// User-availible functions
impl<'a> Query<'a> {
  pub fn set_kind(&mut self, pos: Pos, kind: block::Kind) {
    self.set_block(pos, self.world.block_converter().ty(kind));
  }
  pub fn set_block(&mut self, pos: Pos, ty: block::Type<'a>) {
    self.writes.entry(pos.chunk()).or_insert_with(|| HashMap::new()).insert(pos.chunk_rel(), ty);
  }
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

  fn q_ok<R>(world: &World, f: impl Fn(&mut Query) -> Result<R, QueryError>) {
    world.query(f).unwrap();
  }
  fn q_err<R>(world: &World, f: impl Fn(&mut Query) -> Result<R, QueryError>) -> QueryError {
    match world.query(f) {
      Ok(_) => panic!("query should have failed"),
      Err(e) => e,
    }
  }

  #[test]
  fn basics() {
    let world = World::new_test();
    q_ok(&world, |q| {
      let b = q.get_block(Pos::new(0, 0, 0))?;
      assert_eq!(b.kind(), block::Kind::Stone);

      q.set_kind(Pos::new(0, 0, 0), block::Kind::Air);

      let b = q.get_block(Pos::new(0, 0, 0))?;
      assert_eq!(b.kind(), block::Kind::Stone);

      Ok(())
    });
    // After the above transaction is applied, reads should give a new result
    q_ok(&world, |q| {
      let b = q.get_block(Pos::new(0, 0, 0))?;
      assert_eq!(b.kind(), block::Kind::Air);

      Ok(())
    });
  }

  #[test]
  fn contention() {
    let world = World::new_test();
    q_ok(&world, |q| {
      assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Stone);

      q_ok(&world, |q| {
        assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Stone);

        Ok(())
      });

      Ok(())
    });

    // This should try once, and the inner query will succeed, and the outer query
    // will fail. Then it will try again, and everything should work.
    let tries = std::cell::Cell::new(0);
    q_ok(&world, |q| {
      if tries.get() == 0 {
        assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Stone);
      } else {
        assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Air);
      }

      q_ok(&world, |q| {
        q.set_kind(Pos::new(0, 0, 0), block::Kind::Air);

        Ok(())
      });

      tries.set(tries.get() + 1);
      Ok(())
    });
    assert_eq!(tries.get(), 2);

    assert_eq!(
      q_err(&world, |q| {
        world
          .query(|q| {
            q.set_kind(Pos::new(0, 0, 0), block::Kind::Air);

            Ok(())
          })
          .unwrap();

        assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Stone);

        Ok(())
      }),
      QueryError::Contention
    );
  }
}
