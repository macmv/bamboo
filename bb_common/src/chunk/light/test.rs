use super::{super::paletted::Section, BlockLight, Chunk, LightChunk};
use crate::math::Pos;

fn chunk_from_str(lines: &[&[&str]]) -> (Chunk<Section>, LightChunk<BlockLight>) {
  let mut chunk = Chunk::new(4);
  let mut light = LightChunk::new();

  for (z, plane) in lines.iter().enumerate() {
    for (y, line) in plane.iter().enumerate() {
      for (x, c) in line.chars().enumerate() {
        let pos = Pos::new(x as i32, y as i32, z as i32);
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

#[test]
fn basic_propogate() {
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
      "    #6#    ",
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
  light.set_light(Pos::new(5, 1, 0), 10);
  light.update(&chunk, Pos::new(5, 1, 0));
  assert_eq!(light, expected);
}
