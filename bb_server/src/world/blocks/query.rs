use super::{Block, World};
use crate::{block, world::MultiChunk};
use bb_common::{
  math::{ChunkPos, Pos, PosError, RelPos},
  net::cb,
};
use parking_lot::MutexGuard;
use std::{collections::HashMap, sync::Arc};

pub struct Query<'a> {
  world: &'a Arc<World>,

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
  pub fn query<'a, R>(
    self: &'a Arc<World>,
    f: impl Fn(&mut Query<'a>) -> Result<R, QueryError>,
  ) -> Result<R, QueryError> {
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
  fn new(world: &'a Arc<World>) -> Self {
    Query { world, reads: HashMap::new(), writes: HashMap::new() }
  }

  fn apply(self) -> Result<(), QueryError> {
    // If we didn't write anything, we don't validate anything.
    if self.writes.is_empty() {
      return Ok(());
    }
    // Now that we know some writes are going to be applied, we need to make sure
    // the reads haven't changed while the query was running.
    //
    // This might not be needed, as any chunks that we read and write from will be
    // in the writes list, so this check could probably be skipped.
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
    for (pos, writes) in &self.writes {
      self.world.chunk(*pos, |mut c| {
        if let Some(ver) = self.reads.get(&pos) {
          if *ver != c.version() {
            return Err(QueryError::Contention);
          }
        }
        let mut modified = false;
        for (rel, ty) in writes {
          if self.set_block_inner(
            &mut c,
            pos.block() + Pos::new(rel.x().into(), rel.y().into(), rel.z().into()),
            *ty,
          )? {
            modified = true;
          }
        }
        if modified {
          c.bump_version();
        }
        Ok(())
      })?;
    }
    Ok(())
  }

  fn set_block_inner(
    &self,
    chunk: &mut MultiChunk,
    pos: Pos,
    ty: block::Type,
  ) -> Result<bool, PosError> {
    /*
    let old_ty = chunk.get_type(pos.chunk_rel())?.to_store();
    let old_block = Block::new(self.world, pos, old_ty.ty());
    */
    let new_block = Block::new(self.world, pos, ty);
    let modified = chunk.set_type_no_version_bump(pos.chunk_rel(), ty)?;
    if modified {
      // First, handle the update for the block that was just placed.
      self
        .world
        .world_manager()
        .block_behaviors()
        .call(ty.kind(), |b| b.update_place(self.world, new_block));
      /*
      // After that, handle updates for neighboring blocks.
      macro_rules! dir {
        ( $x:expr, $y:expr, $z:expr ) => {
          if let Ok(ty) = self.world.get_block(pos + Pos::new($x, $y, $z)) {
            self.world.world_manager().block_behaviors().call(ty.kind(), |b| {
              b.update(
                self.world,
                Block::new(self.world, pos + Pos::new($x, $y, $z), ty.ty()),
                old_block,
                new_block,
              )
            });
          }
        };
      }
      dir!(1, 0, 0);
      dir!(-1, 0, 0);
      dir!(0, 1, 0);
      dir!(0, -1, 0);
      dir!(0, 0, 1);
      dir!(0, 0, -1);
      */

      let id = ty.id();
      for p in self.world.players().iter().in_view(pos.chunk()) {
        p.send(cb::packet::BlockUpdate {
          pos,
          state: self.world.block_converter.to_old(id, p.ver().block()),
        });
      }
    }
    Ok(modified)
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
      } else {
        self.reads.insert(pos, c.version());
      }

      f(c)
    })
  }
}
