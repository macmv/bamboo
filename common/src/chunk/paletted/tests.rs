extern crate test;

use super::*;
use test::Bencher;

/// # Initial results
///
/// These are very different results. I think the numbers speak for
/// themselves.
///
/// ```
/// Opt level:        0        |       1      |      2     |
/// Fill manual: ~2,000,000 ns  ~1,200,000 ns   ~100,000ns
/// Fill:        ~9,000 ns      ~5,000 ns       ~300ns
/// ```

#[bench]
fn fill_manual(b: &mut Bencher) {
  let mut s = Section::new();
  let mut block = 0u8;
  b.iter(|| {
    for y in 0..16 {
      for z in 0..16 {
        for x in 0..16 {
          s.set_block(Pos::new(x, y, z), block.into()).unwrap();
        }
      }
    }
    block = block.wrapping_add(1);
  });
}

#[bench]
fn fill(b: &mut Bencher) {
  let mut s = Section::new();
  let mut block = 0u8;
  b.iter(|| {
    s.fill(Pos::new(0, 0, 0), Pos::new(15, 15, 15), block.into()).unwrap();
    block = block.wrapping_add(1);
  });
}

#[test]
fn test_index() {
  let s = Section::default();
  assert_eq!(s.index(Pos::new(0, 0, 0)), (0, 0, 0));
  assert_eq!(s.index(Pos::new(1, 0, 0)), (0, 0, 4));
  assert_eq!(s.index(Pos::new(2, 0, 0)), (0, 0, 8));
  assert_eq!(s.index(Pos::new(0, 0, 1)), (1, 1, 0));
  assert_eq!(s.index(Pos::new(15, 15, 15)), (255, 255, 60));

  let s = Section { bits_per_block: 5, ..Default::default() };
  assert_eq!(s.index(Pos::new(0, 0, 0)), (0, 0, 0));
  assert_eq!(s.index(Pos::new(1, 0, 0)), (0, 0, 5));
  // The id will be split between two longs
  assert_eq!(s.index(Pos::new(12, 0, 0)), (0, 1, 60));
  assert_eq!(s.index(Pos::new(13, 0, 0)), (1, 1, 1));
}
#[test]
fn test_set_palette() {
  let mut s = Section::default();
  // Sanity check
  s.set_palette(Pos::new(0, 0, 0), 0xf);
  assert_eq!(s.data[0], 0xf);
  // Sanity check
  s.set_palette(Pos::new(2, 0, 0), 0xf);
  assert_eq!(s.data[0], 0xf0f);
  // Should work up to the edge of the long
  s.set_palette(Pos::new(15, 0, 0), 0xf);
  assert_eq!(s.data[0], 0xf000000000000f0f);
  // Clearing bits should work
  s.set_palette(Pos::new(15, 0, 0), 0x3);
  assert_eq!(s.data[0], 0x3000000000000f0f);

  let mut s = Section { bits_per_block: 5, ..Default::default() };
  // Sanity check
  s.set_palette(Pos::new(0, 0, 0), 0x1f);
  assert_eq!(s.data[0], 0x1f);
  // Sanity check
  s.set_palette(Pos::new(2, 0, 0), 0x1f);
  assert_eq!(s.data[0], 0x1f << 10 | 0x1f);
  // Should split the id correctly
  s.set_palette(Pos::new(12, 0, 0), 0x1f);
  assert_eq!(s.data[0], 0x1f << 60 | 0x1f << 10 | 0x1f);
  assert_eq!(s.data[1], 0x1f >> 4);
  s.set_palette(Pos::new(25, 0, 0), 0x1f);
  assert_eq!(s.data[1], 0x1f << 61 | 0x1f >> 4);
  assert_eq!(s.data[2], 0x1f >> 3);
  // Clearing bits should work
  s.set_palette(Pos::new(0, 0, 0), 0x3);
  assert_eq!(s.data[0], 0x1f << 60 | 0x1f << 10 | 0x03);
}
#[test]
fn test_get_palette() {
  let mut data = vec![0; 16 * 16 * 16 * 4 / 64];
  data[0] = 0xfaf;
  let s = Section { data, ..Default::default() };
  // Sanity check
  assert_eq!(s.get_palette(Pos::new(0, 0, 0)), 0xf);
  assert_eq!(s.get_palette(Pos::new(1, 0, 0)), 0xa);
  assert_eq!(s.get_palette(Pos::new(2, 0, 0)), 0xf);
  assert_eq!(s.get_palette(Pos::new(3, 0, 0)), 0x0);

  let mut data = vec![0; 16 * 16 * 16 * 4 / 64];
  data[0] = 0x1f << 60 | 0x1f << 10 | 0x1f;
  data[1] = 0x1f >> 4;
  let s = Section { bits_per_block: 5, data, ..Default::default() };
  // Make sure it works with split values
  assert_eq!(s.get_palette(Pos::new(0, 0, 0)), 0x1f);
  assert_eq!(s.get_palette(Pos::new(1, 0, 0)), 0x0);
  assert_eq!(s.get_palette(Pos::new(2, 0, 0)), 0x1f);
  assert_eq!(s.get_palette(Pos::new(12, 0, 0)), 0x1f);
}
#[test]
fn test_increase_bits_per_block() {
  let mut s = Section::default();
  // Place some blocks
  s.set_palette(Pos::new(0, 0, 0), 0xf);
  s.set_palette(Pos::new(1, 0, 0), 0x0);
  s.set_palette(Pos::new(2, 0, 0), 0xf);
  s.set_palette(Pos::new(3, 0, 0), 0xa);
  // We want a split value
  s.set_palette(Pos::new(25, 0, 0), 0xf);

  s.increase_bits_per_block();
  // Sanity check
  assert_eq!(s.bits_per_block, 5);
  // Get blocks should work
  assert_eq!(s.get_palette(Pos::new(0, 0, 0)), 0xf);
  assert_eq!(s.get_palette(Pos::new(1, 0, 0)), 0x0);
  assert_eq!(s.get_palette(Pos::new(2, 0, 0)), 0xf);
  assert_eq!(s.get_palette(Pos::new(3, 0, 0)), 0xa);
  assert_eq!(s.get_palette(Pos::new(25, 0, 0)), 0xf);
  // Make sure the data is correct
  assert_eq!(s.data[0], 0xa << 15 | 0xf << 10 | 0xf);
  assert_eq!(s.data[1], 0xf << 61);
  assert_eq!(s.data[2], 0xf >> 3);
}
#[test]
fn test_insert() {
  // Tests the append part
  let mut s = Section::default();
  assert_eq!(s.insert(5), 1);
  assert_eq!(s.palette, vec![0, 5]);
  assert_eq!(s.block_amounts, vec![4096, 0]);
  assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1)].into_iter().collect());
  assert_eq!(s.insert(10), 2);
  assert_eq!(s.palette, vec![0, 5, 10]);
  assert_eq!(s.block_amounts, vec![4096, 0, 0]);
  assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1), (10, 2)].into_iter().collect());

  // Tests the insert part
  let mut s = Section::default();
  assert_eq!(s.insert(10), 1);
  assert_eq!(s.palette, vec![0, 10]);
  assert_eq!(s.block_amounts, vec![4096, 0]);
  assert_eq!(s.reverse_palette, vec![(0, 0), (10, 1)].into_iter().collect());
  assert_eq!(s.insert(5), 1);
  assert_eq!(s.palette, vec![0, 5, 10]);
  assert_eq!(s.block_amounts, vec![4096, 0, 0]);
  assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1), (10, 2)].into_iter().collect());
}
#[test]
fn test_shift_data_up() -> Result<(), PosError> {
  // Tests shifting all of the block data (this should happen during insert())

  // This section has two blocks placed, one with id 5, and the other with id 10.
  let mut data = vec![0; 4096];
  data[0] = 0x1002;
  let mut s = Section {
    palette: vec![0, 5, 10],
    block_amounts: vec![4096, 0, 0],
    reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
    data,
    ..Default::default()
  };
  // Should shift the block data up.
  s.insert(3);
  assert_eq!(s.data[0], 0x2003);
  // Should shift some of the block data up.
  s.insert(7);
  assert_eq!(s.data[0], 0x2004);
  Ok(())
}
#[test]
fn test_remove() {
  // Tests the pop part
  let mut s = Section {
    palette: vec![0, 5, 10],
    block_amounts: vec![4096, 0, 0],
    reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
    ..Default::default()
  };
  s.remove(2);
  assert_eq!(s.palette, vec![0, 5]);
  assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1)].into_iter().collect());
  s.remove(1);
  assert_eq!(s.palette, vec![0]);
  assert_eq!(s.reverse_palette, vec![(0, 0)].into_iter().collect());

  // Tests the remove part (should affect the elements in the map)
  let mut s = Section {
    palette: vec![0, 5, 10],
    block_amounts: vec![4096, 0, 0],
    reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
    ..Default::default()
  };
  s.remove(1);
  assert_eq!(s.palette, vec![0, 10]);
  assert_eq!(s.reverse_palette, vec![(0, 0), (10, 1)].into_iter().collect());
  s.remove(1);
  assert_eq!(s.palette, vec![0]);
  assert_eq!(s.reverse_palette, vec![(0, 0)].into_iter().collect());
}
#[test]
fn test_shift_data_down() -> Result<(), PosError> {
  // Tests shifting all of the block data (this should happen during insert())

  // This section has one blocks placed with id 10. This is the situation where
  // data has just been modified to no longer contain 5, and we want to remove it
  // from the palette now.
  let mut data = vec![0; 4096];
  data[0] = 0x2;
  let mut s = Section {
    palette: vec![0, 5, 10],
    block_amounts: vec![4096, 0, 0],
    reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
    data,
    ..Default::default()
  };
  // Should shift the block data down.
  s.remove(1);
  assert_eq!(s.data[0], 0x1);
  // Removing 1 again undefined behavior, as 1 is in the block data now. remove()
  // should never be called with the given palette id present.
  Ok(())
}
#[test]
fn test_set_get_block() -> Result<(), PosError> {
  // This tests the entire functionality of set_block, assuming that all above
  // tests passed.

  // Sanity check for palette and block amounts
  let mut s = Section::default();
  s.set_block(Pos::new(0, 0, 0), 5)?;
  assert_eq!(s.block_amounts, vec![4095, 1]);
  assert_eq!(s.palette, vec![0, 5]);

  s.set_block(Pos::new(1, 0, 0), 5)?;
  assert_eq!(s.block_amounts, vec![4094, 2]);
  assert_eq!(s.palette, vec![0, 5]);

  s.set_block(Pos::new(1, 0, 0), 0)?;
  assert_eq!(s.block_amounts, vec![4095, 1]);
  assert_eq!(s.palette, vec![0, 5]);

  s.set_block(Pos::new(0, 0, 0), 0)?;
  assert_eq!(s.block_amounts, vec![4096]);
  assert_eq!(s.palette, vec![0]);

  // Make sure that higher palette ids get shifted down correctly.
  let mut s = Section::default();
  s.set_block(Pos::new(0, 0, 0), 10)?;
  assert_eq!(s.block_amounts, vec![4095, 1]);
  assert_eq!(s.palette, vec![0, 10]);

  // 5 should be inserted in the middle
  s.set_block(Pos::new(1, 0, 0), 5)?;
  assert_eq!(s.block_amounts, vec![4094, 1, 1]);
  assert_eq!(s.palette, vec![0, 5, 10]);

  // 10 should be shifted down
  s.set_block(Pos::new(1, 0, 0), 0)?;
  assert_eq!(s.block_amounts, vec![4095, 1]);
  assert_eq!(s.palette, vec![0, 10]);

  // Default state
  s.set_block(Pos::new(0, 0, 0), 0)?;
  assert_eq!(s.block_amounts, vec![4096]);
  assert_eq!(s.palette, vec![0]);

  // Make sure that replacing blocks works
  let mut s = Section::default();
  s.set_block(Pos::new(1, 0, 0), 5)?;
  assert_eq!(s.palette, vec![0, 5]);
  assert_eq!(s.block_amounts, vec![4095, 1]);
  s.set_block(Pos::new(1, 0, 0), 10)?;
  assert_eq!(s.palette, vec![0, 10]);
  assert_eq!(s.block_amounts, vec![4095, 1]);

  // Test get block
  let mut s = Section::default();
  s.set_block(Pos::new(0, 0, 0), 10)?;
  assert_eq!(s.get_block(Pos::new(0, 0, 0))?, 10);
  s.set_block(Pos::new(0, 0, 0), 123)?;
  dbg!(&s.reverse_palette);
  assert_eq!(s.get_block(Pos::new(0, 0, 0))?, 123);
  s.set_block(Pos::new(1, 3, 2), 5)?;
  assert_eq!(s.get_block(Pos::new(1, 3, 2))?, 5);
  s.set_block(Pos::new(15, 15, 15), 420)?;
  assert_eq!(s.get_block(Pos::new(15, 15, 15))?, 420);

  Ok(())
}
#[test]
fn test_set_all() -> Result<(), PosError> {
  let mut s = Section::default();
  for x in 0..16 {
    for y in 0..16 {
      for z in 0..16 {
        s.set_block(Pos::new(x, y, z), 20)?;
      }
    }
  }
  assert_eq!(s.data, vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
  assert_eq!(s.palette, vec![0, 20]);
  assert_eq!(s.block_amounts, vec![0, 4096]);

  s.set_block(Pos::new(0, 0, 0), 5)?;

  let mut data = vec![0x2222222222222222; 16 * 16 * 16 * 4 / 64];
  data[0] = 0x2222222222222221;
  assert_eq!(s.data, data);
  assert_eq!(s.palette, vec![0, 5, 20]);
  assert_eq!(s.block_amounts, vec![0, 1, 4095]);

  Ok(())
}
#[test]
fn test_fill() -> Result<(), PosError> {
  let mut s = Section::default();
  s.fill(Pos::new(0, 0, 0), Pos::new(15, 15, 15), 20)?;
  assert_eq!(s.data, vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
  assert_eq!(s.palette, vec![0, 20]);
  assert_eq!(s.block_amounts, vec![0, 4096]);

  s.set_block(Pos::new(0, 0, 0), 5)?;

  let mut data = vec![0x2222222222222222; 16 * 16 * 16 * 4 / 64];
  data[0] = 0x2222222222222221;
  assert_eq!(s.data, data);
  assert_eq!(s.palette, vec![0, 5, 20]);
  assert_eq!(s.block_amounts, vec![0, 1, 4095]);

  let mut s = Section::default();
  s.fill(Pos::new(3, 4, 5), Pos::new(8, 9, 10), 20)?;

  dbg!(&s);
  for x in 0..16 {
    for y in 0..16 {
      for z in 0..16 {
        let expected =
          if x >= 3 && x <= 8 && y >= 4 && y <= 9 && z >= 5 && z <= 10 { 20 } else { 0 };
        assert_eq!(s.get_block(Pos::new(x, y, z))?, expected);
      }
    }
  }
  assert_eq!(s.block_amounts[0] + s.block_amounts[1], 4096);

  Ok(())
}

#[test]
fn test_from_proto() {
  let mut pb = proto::chunk::Section::default();
  pb.bits_per_block = 4;
  pb.data = vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64];
  pb.palette.push(0);
  pb.palette.push(5);

  let s = Section::from_latest_proto(pb.clone());
  assert_eq!(s.data, vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
  assert_eq!(s.palette, vec![0, 5]);
  assert_eq!(s.block_amounts, vec![0, 4096]);

  let s = Section::from_old_proto(pb, &|val| val + 5);
  assert_eq!(s.data, vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
  assert_eq!(s.palette, vec![5, 10]);
  assert_eq!(s.block_amounts, vec![0, 4096]);
}
