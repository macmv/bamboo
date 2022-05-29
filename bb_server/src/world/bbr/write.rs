//! Implements `MessageWrite` for `Region`, and `Region::save`.

use super::Region;
use crate::world::CountedChunk;
use bb_transfer::{MessageWrite, MessageWriter, WriteError};
use std::{fs, fs::File, io::Write};

impl Region {
  pub fn save(&self) {
    use std::cell::RefCell;
    thread_local! {
      static CACHE: (RefCell<Vec<u8>>, RefCell<Vec<u8>>) = (RefCell::new(vec![]), RefCell::new(vec![]));
    }
    CACHE.with(|(region_cache, compression_cache)| {
      let mut region_cache = region_cache.borrow_mut();
      let mut compression_cache = compression_cache.borrow_mut();

      region_cache.clear();
      let mut writer = MessageWriter::<&mut Vec<u8>>::new(&mut region_cache);
      writer.write(&self).unwrap();

      use bb_common::flate2::{write::GzEncoder, Compression};

      compression_cache.clear();
      let mut encoder =
        GzEncoder::<&mut Vec<u8>>::new(&mut compression_cache, Compression::default());
      encoder.write_all(&region_cache).unwrap();
      encoder.finish().unwrap();

      // TODO: Warn about errors here
      fs::create_dir_all("world/chunks").unwrap();
      let fname = format!("world/chunks/{}.{}.bbr", self.pos.x, self.pos.z);
      File::create(fname).unwrap().write_all(&compression_cache).unwrap();
    });
  }
}

impl MessageWrite for Region {
  fn write<W: Write>(&self, w: &mut MessageWriter<W>) -> Result<(), WriteError> {
    w.write_struct(1024, |w| {
      for chunk in &self.chunks {
        w.write(&chunk.as_ref().map(WriteableChunk))?;
      }
      Ok(())
    })
  }
}

struct WriteableChunk<'a>(&'a CountedChunk);

impl MessageWrite for WriteableChunk<'_> {
  fn write<W: Write>(&self, w: &mut MessageWriter<W>) -> Result<(), WriteError> {
    // TODO: Write light
    w.write_struct(1, |w| {
      w.write_list(self.0.chunk.lock().inner().sections())?;
      Ok(())
    })
  }
}
