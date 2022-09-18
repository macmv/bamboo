use super::{super::MultiChunk, World};
use crate::block;
use bb_common::math::{ChunkPos, Pos, PosError};
use parking_lot::RwLockReadGuard;
use std::collections::HashMap;

pub struct Query<'a> {
  world: &'a World,

  reads:  HashMap<ChunkPos, u32>,
  writes: HashMap<Pos, block::Type<'a>>,
}

struct VersionedReadLock<'a> {
  version: u32,
  lock:    RwLockReadGuard<'a, MultiChunk>,
}

/// We will try each query 3 times before failing
const CONTENTION_LIMIT: u32 = 3;

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
  pub fn query<R>(&self, f: impl Fn(&mut Query) -> Result<R, PosError>) -> Result<R, QueryError> {
    for _ in 0..CONTENTION_LIMIT {
      let mut query = Query::new(self);
      let res = f(&mut query)?;
      match query.apply() {
        Ok(()) => return Ok(res),
        Err(QueryError::Contention) => continue,
        Err(e) => return Err(e),
      }
    }
    Err(QueryError::Contention)
  }
}

impl<'a> Query<'a> {
  fn new(world: &'a World) -> Self {
    Query { world, reads: HashMap::new(), writes: HashMap::new() }
  }

  fn apply(self) -> Result<(), QueryError> { Ok(()) }
}

/// User-availible functions
impl<'a> Query<'a> {
  pub fn set_block(&mut self, pos: Pos, ty: block::Type<'a>) { self.writes.insert(pos, ty); }
  pub fn get_block(
    &mut self,
    pos: Pos,
    ty: block::Type<'a>,
  ) -> Result<block::Type<'a>, QueryError> {
    let current_version = self.reads.get(&pos.chunk());
    self.world.chunk(pos.chunk(), |c| {
      if let Some(current) = current_version {
        if c.version() > current {
          return Err(QueryError::Contention);
        }
      }
      self.reads.insert(pos.chunk(), c.version());

      Ok(c.get_type(pos.chunk_rel())?)
    })
  }
}
