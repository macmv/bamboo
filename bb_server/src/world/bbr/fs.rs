//! Implements `MessageWrite` and `MessageRead` for `Region`, `Region::save`,
//! and `Region::load`.

use super::Region;
use crate::world::{CountedChunk, MultiChunk};
use bb_common::{
  chunk::paletted,
  flate2::{read::GzDecoder, write::GzEncoder, Compression},
};
use bb_transfer::{
  MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError, StructRead, StructReader,
  WriteError,
};
use std::{
  cell::RefCell,
  fs,
  fs::File,
  io::{Read, Write},
  path::PathBuf,
};

thread_local! {
  static CACHE: (RefCell<Vec<u8>>, RefCell<Vec<u8>>) = (RefCell::new(vec![]), RefCell::new(vec![]));
}

impl Region {
  /// Writes all the stored chunks to disk.
  pub(super) fn save(&self) {
    CACHE.with(|(region_cache, compression_cache)| {
      let mut region_cache = region_cache.borrow_mut();
      let mut compression_cache = compression_cache.borrow_mut();

      region_cache.clear();
      let mut writer = MessageWriter::<&mut Vec<u8>>::new(&mut region_cache);
      writer.write(&self).unwrap();

      compression_cache.clear();
      let mut encoder =
        GzEncoder::<&mut Vec<u8>>::new(&mut compression_cache, Compression::default());
      encoder.write_all(&region_cache).unwrap();
      encoder.finish().unwrap();

      // TODO: Warn about errors here
      let path = self.fname();
      debug!("saving region to {}", path.display());
      self.print_summary();
      fs::create_dir_all(path.parent().unwrap()).unwrap();
      File::create(path).unwrap().write_all(&compression_cache).unwrap();
    });
  }

  /// Overwrites all stored chunks with the file on disk, if present. If not
  /// present, this will clear all loaded chunks.
  pub(super) fn load(&mut self) {
    CACHE.with(|(region_cache, compression_cache)| {
      let mut region_cache = region_cache.borrow_mut();
      let mut compression_cache = compression_cache.borrow_mut();

      let path = self.fname();
      if path.exists() {
        debug!("loading region from {}", path.display());
        compression_cache.clear();
        let n = File::open(path).unwrap().read_to_end(&mut compression_cache).unwrap();

        let mut decoder = GzDecoder::<&[u8]>::new(&compression_cache[..n]);
        region_cache.clear();
        let n = match decoder.read_to_end(&mut region_cache) {
          Ok(n) => n,
          Err(e) => {
            warn!("couldn't read chunk: {e}");
            return;
          }
        };

        let mut reader = MessageReader::new(&region_cache[..n]);
        let data: RegionData = reader.read_struct().unwrap();
        for (chunk, data) in self.chunks.iter_mut().zip(data.0.into_iter()) {
          if let Some(data) = data {
            if let Some(chunk) = chunk {
              data.update_chunk(chunk);
            } else {
              let mut c = CountedChunk::new(MultiChunk::new(self.wm.clone(), true));
              data.update_chunk(&mut c);
              *chunk = Some(c);
            }
          } else {
            *chunk = None;
          }
        }

        self.print_summary();
      }
    });
  }

  fn print_summary(&self) {
    println!("CHUNK AT {} {}", self.pos.x, self.pos.z);
    for z in 0..32 {
      for x in 0..32 {
        if self.has_chunk(super::RegionRelPos { x, z }) {
          print!("x");
        } else {
          print!(".");
        }
      }
      println!();
    }
  }

  fn fname(&self) -> PathBuf {
    PathBuf::new().join("world").join("chunks").join(&format!("{}.{}.bbr", self.pos.x, self.pos.z))
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

#[derive(Debug)]
struct RegionData([Option<ReadableChunk>; 1024]);
impl StructRead<'_> for RegionData {
  fn read_struct(mut r: StructReader) -> Result<Self, ReadError> {
    const NONE: Option<ReadableChunk> = None;
    let mut chunks = [NONE; 1024];
    for i in 0..1024 {
      chunks[i] = r.must_read(i as u64)?;
    }
    Ok(RegionData(chunks))
  }
}
#[derive(Debug)]
struct ReadableChunk(Vec<Option<paletted::Section>>);
impl MessageRead<'_> for ReadableChunk {
  fn read(r: &mut MessageReader) -> Result<Self, ReadError> { r.read_struct() }
}
impl StructRead<'_> for ReadableChunk {
  fn read_struct(mut r: StructReader) -> Result<Self, ReadError> {
    Ok(ReadableChunk(r.must_read(0)?))
  }
}

impl ReadableChunk {
  pub fn update_chunk(self, chunk: &mut CountedChunk) {
    let mut lock = chunk.lock();
    for (y, section) in self.0.into_iter().enumerate() {
      if let Some(s) = section {
        *lock.inner_mut().section_mut(y as u32) = s;
      }
    }
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
