use super::query::{Query, QueryError};
use crate::{block, world::World};
use bb_common::math::Pos;
use std::{cell::Cell, sync::Arc};

#[track_caller]
fn q_ok<R>(world: &Arc<World>, f: impl Fn(&mut Query) -> Result<R, QueryError>) {
  world.query(f).unwrap();
}
#[track_caller]
fn q_tries<R>(
  world: &Arc<World>,
  expected: u32,
  f: impl Fn(u32, &mut Query) -> Result<R, QueryError>,
) -> R {
  let tries = Cell::new(0);
  match world.query(|q| {
    let res = f(tries.get(), q);
    tries.set(tries.get() + 1);
    res
  }) {
    Ok(res) => {
      assert_eq!(
        tries.get(),
        expected,
        "query took {} tries, was expecting {}",
        tries.get(),
        expected
      );
      res
    }
    Err(e) => panic!("query should not have failed: {e:?}"),
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
  q_tries(&world, 1, |_, q| {
    assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Air);

    q_ok(&world, |q| {
      assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Air);

      Ok(())
    });

    Ok(())
  });

  // This should try once, and the inner query will succeed, and because the outer
  // query isn't writing, it will also succeed.
  q_tries(&world, 1, |_, q| {
    assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Air);

    q_ok(&world, |q| {
      q.set_kind(Pos::new(0, 0, 0), block::Kind::Stone);

      Ok(())
    });

    Ok(())
  });

  // This should try once, and the inner query will succeed, and then the second
  // read in the outer query will cause the first attempt to fail.
  q_tries(&world, 2, |tries, q| {
    if tries == 0 {
      assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Stone);
    } else {
      assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Air);
    }

    q_ok(&world, |q| {
      q.set_kind(Pos::new(0, 0, 0), block::Kind::Air);

      Ok(())
    });

    // This read is the only reason the outer query fails.
    assert_eq!(q.get_kind(Pos::new(0, 0, 0))?, block::Kind::Air);

    Ok(())
  });
}
