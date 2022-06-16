use super::{super::paletted::Section, BlockLight, Chunk, LightChunk};
use crate::math::RelPos;
use pretty_assertions::assert_eq;

#[track_caller]
fn chunk_from_str(lines: &[&[&str]]) -> (Chunk<Section>, LightChunk<BlockLight>) {
  let mut chunk = Chunk::new(4);
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
        chunk.set_block(pos, if c == '#' { 1 } else { 0 }).unwrap();
        let mut tmp = [0u8; 1];
        let string = c.encode_utf8(&mut tmp);
        light.set_light(
          pos,
          if c != '#' && c != ' ' { u8::from_str_radix(string, 16).unwrap() } else { 0 },
        );
      }
    }
  }

  (chunk, light)
}

fn assert_plane_matches(a: &mut LightChunk<BlockLight>, b: &mut LightChunk<BlockLight>) {
  let mut a_str = String::new();
  for y in 0..16 {
    for x in 0..16 {
      let v = a.get_light(RelPos::new(x, y, 0));
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
      let v = b.get_light(RelPos::new(x, y, 0));
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
  #[rustfmt::skip]
  let (chunk, expected) = chunk_from_str(&[
    &[
      "    ###    ",
      "    #a#    ",
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

  let mut light = LightChunk::new();
  light.set_light(RelPos::new(5, 1, 0), 10);
  light.update(&chunk, RelPos::new(5, 1, 0));
  assert_eq!(light, expected);

  light.set_light(RelPos::new(10, 11, 0), 0xa);
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
    "          a     ",
    "                ",
    "                ",
    "                ",
    "                ",
  ]]);
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
}
