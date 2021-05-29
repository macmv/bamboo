mod buffer;
pub mod chat;

pub use buffer::{Buffer, BufferError};
pub use chat::Chat;

pub fn serialize_varint(v: i32) -> Vec<u8> {
  // Need to work with u32, as >> acts differently on i32 vs u32.
  let mut val = v as u32;
  let mut out = vec![];
  for _ in 0..5 {
    let mut b: u8 = val as u8 & 0b01111111;
    val >>= 7;
    if val != 0 {
      b |= 0b10000000;
    }
    out.push(b);
    if val == 0 {
      break;
    }
  }
  out
}

pub fn read_varint(buf: &[u8]) -> (i32, isize) {
  let mut res: i32 = 0;
  let mut total_read: isize = 0;
  for i in 0..5 {
    if i >= buf.len() {
      // Incomplete varint
      return (0, 0);
    }
    let read = buf[i];
    if i == 4 && read & 0b10000000 != 0 {
      // Invalid varint (read < 0 means invalid varint)
      return (0, -1);
    }

    let v = read & 0b01111111;
    res |= (v as i32) << (7 * i);

    if read & 0b10000000 == 0 {
      // Done reading bytes, so we set total read
      total_read = i as isize + 1;
      break;
    }
  }
  (res, total_read)
}
