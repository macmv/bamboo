//! Implements `MessageWrite` for `Region`, and `Region::save`.

use super::Region;
use crate::world::CountedChunk;
use bb_transfer::{MessageWrite, MessageWriter, WriteError};

impl Region {
  pub fn save(&self) {
    use std::cell::RefCell;
    thread_local! {
      static REGION_CACHE: RefCell<Vec<u8>> = RefCell::new(vec![]);
    }
    REGION_CACHE.with(|cache| {
      let mut cache = cache.borrow_mut();
      let mut writer = MessageWriter::new(&mut cache);
      // TODO: Writer which appends
      writer.write(&self).unwrap();
    });
  }
}

impl MessageWrite for Region {
  fn write(&self, w: &mut MessageWriter) -> Result<(), WriteError> {
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
  fn write(&self, w: &mut MessageWriter) -> Result<(), WriteError> {
    // TODO: Write light
    w.write_struct(1, |w| {
      w.write_list(self.0.chunk.lock().inner().sections())?;
      Ok(())
    })
  }
}
