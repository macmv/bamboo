use super::BlockLightChunk;
use crate::{
  block,
  world::{BlockData, WorldManager},
};
use bb_common::{chunk::LightChunk, math::RelPos};
use pretty_assertions::assert_eq;
use std::sync::Arc;

#[track_caller]
fn chunk_from_str(wm: Arc<WorldManager>, lines: &[&[&str]]) -> (BlockData, BlockLightChunk) {
  let mut chunk = BlockData::new(wm, 256, 0);
  let mut light = LightChunk::new();

  if lines.len() > 16 {
    panic!("lines too long");
  }
  for (z, plane) in lines.iter().enumerate() {
    if plane.len() > 16 {
      panic!("plane at {z} too long");
    }
    for (y, line) in plane.iter().enumerate() {
      if line.len() > 16 {
        panic!("line at {z} {y} too long (len: {})", line.len());
      }
      for (x, c) in line.chars().enumerate() {
        let pos = RelPos::new(x as u8, y as i32, z as u8);
        chunk
          .set_kind(
            pos,
            match c {
              '#' => block::Kind::Stone,
              // This emits light level 10, which is what I use in the tests below.
              'T' => block::Kind::CryingObsidian,
              _ => block::Kind::Air,
            },
          )
          .unwrap();
        let mut tmp = [0u8; 1];
        let string = c.encode_utf8(&mut tmp);
        light.set_light(
          pos,
          match c {
            '#' | ' ' => 0,
            'T' => 10,
            _ => u8::from_str_radix(string, 16).unwrap(),
          },
        );
      }
    }
  }

  (chunk, BlockLightChunk { data: light })
}

#[track_caller]
fn assert_plane_matches(a: &mut BlockLightChunk, b: &mut BlockLightChunk) {
  let mut a_str = String::new();
  for y in 0..16 {
    for x in 0..16 {
      let v = a.data.get_light(RelPos::new(x, y, 0));
      if v == 0 {
        a_str.push('.');
      } else {
        a_str.push_str(&format!("{v:x}"));
      }
    }
    a_str.push('\n');
  }
  let mut b_str = String::new();
  for y in 0..16 {
    for x in 0..16 {
      let v = b.data.get_light(RelPos::new(x, y, 0));
      if v == 0 {
        b_str.push('.');
      } else {
        b_str.push_str(&format!("{v:x}"));
      }
    }
    b_str.push('\n');
  }
  assert_eq!(a_str, b_str);
}

#[test]
fn basic_propagate() {
  let wm = Arc::new(WorldManager::new(false));

  #[rustfmt::skip]
  let (chunk, expected) = chunk_from_str(wm, &[
    &[
      "    ###    ",
      "    #T#    ",
      "    #9#    ",
      "    #8#    ",
      "   1#7#1   ",
      "  12#6#21  ",
      " 123454321 ",
      "  1234321  ",
      "   12321   ",
      "    121    ",
      "     1     ",
    ],
    &[
      "    ###    ",
      "    #9#    ",
      "    #8#    ",
      "    #7#    ",
      "    #6#    ",
      "   1#5#1   ",
      "  1234321  ",
      "   12321   ",
      "    121    ",
      "     1     ",
    ],
    &[
      "    ###    ",
      "    #8#    ",
      "    #7#    ",
      "    #6#    ",
      "    #5#    ",
      "    #4#    ",
      "   12321   ",
      "    121    ",
      "     1     ",
    ],
    &[
      "    ###    ",
      "    #7#    ",
      "    #6#    ",
      "    #5#    ",
      "    #4#    ",
      "    #3#    ",
      "    121    ",
      "     1     ",
    ],
    &[
      "    ###    ",
      "    #6#    ",
      "    #5#    ",
      "    #4#    ",
      "    #3#    ",
      "    #2#    ",
      "     1     ",
    ],
    &[
      "    ###    ",
      "    #5#    ",
      "    #4#    ",
      "    #3#    ",
      "    #2#    ",
      "    #1#    "
    ],
    &[
      "    ###    ",
      "    #4#    ",
      "    #3#    ",
      "    #2#    ",
      "    #1#    ",
    ],
    &[
      "    ###    ",
      "    #3#    ",
      "    #2#    ",
      "    #1#    ",
    ],
    &[
      "    ###    ",
      "    #2#    ",
      "    #1#    ",
    ],
    &[
      "    ###    ",
      "    #1#    ",
    ],
  ]);

  let mut light = BlockLightChunk::new();
  light.update(&chunk, RelPos::new(5, 1, 0));
  assert_eq!(light, expected);

  /*
    light.data.set_light(RelPos::new(10, 11, 0), 0xa);
    let (_, mut expected) = chunk_from_str(
      wm.clone(),
      &[&[
        "    ###         ",
        "    #a#         ",
        "    #9#         ",
        "    #8#         ",
        "   1#7#1        ",
        "  12#6#21       ",
        " 123454321      ",
        "  1234321       ",
        "   12321        ",
        "    121         ",
        "     1          ",
        "          a     ",
        "                ",
        "                ",
        "                ",
        "                ",
      ]],
    );
    assert_plane_matches(&mut light, &mut expected);

    light.update(&chunk, RelPos::new(10, 11, 0));
    let (_, mut expected) = chunk_from_str(&[&[
      "    ###         ",
      "    #a#         ",
      "    #9#   1     ",
      "    #8#  121    ",
      "   1#7#112321   ",
      "  12#6#2234321  ",
      " 12345433454321 ",
      "  12343345654321",
      "   1233456765432",
      "   1234567876543",
      "  12345678987654",
      " 123456789a98765",
      "  12345678987654",
      "   1234567876543",
      "    123456765432",
      "     12345654321",
    ]]);
    assert_plane_matches(&mut light, &mut expected);

    light.set_light(RelPos::new(10, 11, 0), 0x0);
    let (_, mut expected) = chunk_from_str(&[&[
      "    ###         ",
      "    #a#         ",
      "    #9#   1     ",
      "    #8#  121    ",
      "   1#7#112321   ",
      "  12#6#2234321  ",
      " 12345433454321 ",
      "  12343345654321",
      "   1233456765432",
      "   1234567876543",
      "  12345678987654",
      " 123456789 98765",
      "  12345678987654",
      "   1234567876543",
      "    123456765432",
      "     12345654321",
    ]]);
    assert_plane_matches(&mut light, &mut expected);

    light.update(&chunk, RelPos::new(10, 11, 0));
    let (_, mut expected) = chunk_from_str(&[&[
      "    ###         ",
      "    #a#         ",
      "    #9#         ",
      "    #8#         ",
      "   1#7#1        ",
      "  12#6#21       ",
      " 123454321      ",
      "  1234321       ",
      "   12321        ",
      "    121         ",
      "     1          ",
      "                ",
      "                ",
      "                ",
      "                ",
      "                ",
    ]]);
    assert_plane_matches(&mut light, &mut expected);
  */
}

/*
#[test]
fn remove_light() {
  let (chunk, _) = chunk_from_str(&[]);

  let mut light = LightChunk::new();
  light.set_light(RelPos::new(5, 1, 0), 0xa);
  light.set_light(RelPos::new(6, 1, 0), 0x9);

  let (_, mut expected) = chunk_from_str(&[&["                ", "     a9         "]]);
  assert_plane_matches(&mut light, &mut expected);

  light.set_light(RelPos::new(5, 1, 0), 0x0);
  let (_, mut expected) = chunk_from_str(&[&["                ", "      9         "]]);
  assert_plane_matches(&mut light, &mut expected);

  light.update(&chunk, RelPos::new(5, 1, 0));
  let (_, mut expected) = chunk_from_str(&[&[
    "23456787654321  ",
    "345678987654321 ",
    "23456787654321  ",
    "1234567654321   ",
    " 12345654321    ",
    "  123454321     ",
    "   1234321      ",
    "    12321       ",
    "     121        ",
    "      1         ",
  ]]);
  assert_plane_matches(&mut light, &mut expected);
}
*/
