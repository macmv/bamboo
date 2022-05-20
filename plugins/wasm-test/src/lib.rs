use bb_plugin::{
  chunk::{paletted, Chunk},
  util::{chat::Color, Chat},
};

#[macro_use]
extern crate bb_plugin;

#[no_mangle]
extern "C" fn init() {
  bb_plugin::init();
  bb_plugin::set_on_block_place(|player, pos| {
    let mut chat = Chat::new("player: ");
    chat.add(&player.username());
    chat.add(", x: ").color(Color::Red);
    chat.add(&format!("{}, ", pos.x));
    chat.add("y: ").color(Color::Red);
    chat.add(&format!("{}, ", pos.y));
    chat.add("z: ").color(Color::Red);
    chat.add(&format!("{}", pos.z));
    let bb = bb_plugin::instance();
    bb.broadcast(chat);
    info!("hello world");
  });
  bb_plugin::add_world_generator("testing-generator", |chunk, pos| {
    for col in pos.columns() {
      let height = (noise(col.x(), col.z()) * 20.0 + 10.0) as i32;
      for y in 0..height {
        chunk.set_block(col.chunk_rel().add_y(y), 1);
      }
    }
  });
}

fn noise(x: i32, y: i32) -> f32 {
  let sec_x = x / 64;
  let sec_y = y / 64;
  let height = (random(sec_x as u64 + (sec_y as u64) << 32) % 1024) as f32 / 1024.0;
  let rel_x = 32 - ((x % 64) - 32).abs();
  let rel_y = 32 - ((y % 64) - 32).abs();
  ((rel_x as f32).powi(2) + (rel_y as f32).powi(2)).sqrt() * height / 32.0
}

fn random(mut seed: u64) -> u64 {
  seed = seed.wrapping_add(0x60bee2bee120fc15);
  let mut tmp = (seed as u128).wrapping_mul(0xa3b195354a39b70d);
  tmp = ((tmp >> 64) as u64 ^ tmp as u64) as u128;
  return ((tmp >> 64) as u64 ^ tmp as u64) as u64;
}
