use super::*;

const MAX_BPE: u8 = 15;

#[test]
fn test_index() {
  let s = Section::new(MAX_BPE);
  assert_eq!(s.index(Pos::new(0, 0, 0)), 0);
  assert_eq!(s.index(Pos::new(1, 0, 0)), 1);
  assert_eq!(s.index(Pos::new(2, 0, 0)), 2);
  assert_eq!(s.index(Pos::new(0, 0, 1)), 16);
  assert_eq!(s.index(Pos::new(15, 15, 15)), 4095);
}
#[test]
fn test_increase_bits_per_block() {
  unsafe {
    let mut s = Section::new(MAX_BPE);
    // Place some blocks
    s.set_palette(Pos::new(0, 0, 0), 0xf);
    s.set_palette(Pos::new(1, 0, 0), 0x0);
    s.set_palette(Pos::new(2, 0, 0), 0xf);
    s.set_palette(Pos::new(3, 0, 0), 0xa);
    // We want a split value
    s.set_palette(Pos::new(25, 0, 0), 0xf);

    s.data.increase_bpe(1);
    // Sanity check
    assert_eq!(s.data.bpe(), 5);
    // Get blocks should work
    assert_eq!(s.get_palette(Pos::new(0, 0, 0)), 0xf);
    assert_eq!(s.get_palette(Pos::new(1, 0, 0)), 0x0);
    assert_eq!(s.get_palette(Pos::new(2, 0, 0)), 0xf);
    assert_eq!(s.get_palette(Pos::new(3, 0, 0)), 0xa);
    assert_eq!(s.get_palette(Pos::new(25, 0, 0)), 0xf);
  }
}
#[test]
fn test_insert() {
  // Tests the append part
  let mut s = Section::new(MAX_BPE);
  assert_eq!(s.insert(5), 1);
  assert_eq!(s.palette, vec![0, 5]);
  assert_eq!(s.block_amounts, vec![4096, 0]);
  assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1)].into_iter().collect());
  assert_eq!(s.insert(10), 2);
  assert_eq!(s.palette, vec![0, 5, 10]);
  assert_eq!(s.block_amounts, vec![4096, 0, 0]);
  assert_eq!(s.reverse_palette, vec![(0, 0), (5, 1), (10, 2)].into_iter().collect());

  // Tests the insert part
  let mut s = Section::new(MAX_BPE);
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
  let mut data = vec![0; 4096 * 4 / 64];
  data[0] = 0x1002;
  let mut s = Section {
    palette: vec![0, 5, 10],
    block_amounts: vec![4096, 0, 0],
    reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
    data: BitArray::from_data(4, data),
    ..Section::new(MAX_BPE)
  };
  // Should shift the block data up.
  s.insert(3);
  assert_eq!(s.data.clone_inner()[0], 0x2003);
  // Should shift some of the block data up.
  s.insert(7);
  assert_eq!(s.data.clone_inner()[0], 0x2004);
  Ok(())
}
#[test]
fn test_remove() {
  // Tests the pop part
  let mut s = Section {
    palette: vec![0, 5, 10],
    block_amounts: vec![4096, 0, 0],
    reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
    ..Section::new(MAX_BPE)
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
    ..Section::new(MAX_BPE)
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
  let mut data = vec![0; 4096 * 4 / 64];
  data[0] = 0x2;
  let mut s = Section {
    palette: vec![0, 5, 10],
    block_amounts: vec![4096, 0, 0],
    reverse_palette: vec![(0, 0), (5, 1), (10, 2)].into_iter().collect(),
    data: BitArray::from_data(4, data),
    ..Section::new(MAX_BPE)
  };
  // Should shift the block data down.
  s.remove(1);
  assert_eq!(s.data.clone_inner()[0], 0x1);
  // Removing 1 again undefined behavior, as 1 is in the block data now. remove()
  // should never be called with the given palette id present.
  Ok(())
}
#[test]
fn test_set_get_block() -> Result<(), PosError> {
  // This tests the entire functionality of set_block, assuming that all above
  // tests passed.

  // Sanity check for palette and block amounts
  let mut s = Section::new(MAX_BPE);
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
  let mut s = Section::new(MAX_BPE);
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
  let mut s = Section::new(MAX_BPE);
  s.set_block(Pos::new(1, 0, 0), 5)?;
  assert_eq!(s.palette, vec![0, 5]);
  assert_eq!(s.block_amounts, vec![4095, 1]);
  s.set_block(Pos::new(1, 0, 0), 10)?;
  assert_eq!(s.palette, vec![0, 10]);
  assert_eq!(s.block_amounts, vec![4095, 1]);

  // Test get block
  let mut s = Section::new(MAX_BPE);
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
  let mut s = Section::new(MAX_BPE);
  for x in 0..16 {
    for y in 0..16 {
      for z in 0..16 {
        s.set_block(Pos::new(x, y, z), 20)?;
      }
    }
  }
  assert_eq!(s.data.clone_inner(), vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
  assert_eq!(s.palette, vec![0, 20]);
  assert_eq!(s.block_amounts, vec![0, 4096]);

  s.set_block(Pos::new(0, 0, 0), 5)?;

  let mut data = vec![0x2222222222222222; 16 * 16 * 16 * 4 / 64];
  data[0] = 0x2222222222222221;
  assert_eq!(s.data.clone_inner(), data);
  assert_eq!(s.palette, vec![0, 5, 20]);
  assert_eq!(s.block_amounts, vec![0, 1, 4095]);

  Ok(())
}
#[test]
fn test_fill() -> Result<(), PosError> {
  let mut s = Section::new(MAX_BPE);
  s.fill(Pos::new(0, 0, 0), Pos::new(15, 15, 15), 20)?;
  assert_eq!(s.data.clone_inner(), vec![0x1111111111111111; 16 * 16 * 16 * 4 / 64]);
  assert_eq!(s.palette, vec![0, 20]);
  assert_eq!(s.block_amounts, vec![0, 4096]);

  s.set_block(Pos::new(0, 0, 0), 5)?;

  let mut data = vec![0x2222222222222222; 16 * 16 * 16 * 4 / 64];
  data[0] = 0x2222222222222221;
  assert_eq!(s.data.clone_inner(), data);
  assert_eq!(s.palette, vec![0, 5, 20]);
  assert_eq!(s.block_amounts, vec![0, 1, 4095]);

  let mut s = Section::new(MAX_BPE);
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
