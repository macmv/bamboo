//! Implements `MessageWrite` and `MessageRead` for `Region`, `Region::save`,
//! and `Region::load`.

use super::Region;
use crate::world::{CountedChunk, MultiChunk};
use bb_common::{
  chunk::{paletted, Section},
  flate2::{read::GzDecoder, write::GzEncoder, Compression},
  version::BlockVersion,
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
              let mut c = CountedChunk::new(MultiChunk::new(
                self.world.world_manager().clone(),
                true,
                self.world.height,
                self.world.min_y,
              ));
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

  pub(super) fn print_summary(&self) {
    /*
    println!("CHUNK AT {} {}", self.pos.x, self.pos.z);
    for z in 0..32 {
      for x in 0..32 {
        if let Some(c) = self.get(super::RegionRelPos { x, z }) {
          let count = c.count.load(std::sync::atomic::Ordering::SeqCst);
          if count > 0 {
            print!("{count}");
          } else {
            print!("x");
          }
        } else {
          print!(".");
        }
      }
      println!();
    }
    */
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
    for (i, chunk) in chunks.iter_mut().enumerate() {
      *chunk = r.must_read(i as u64)?;
    }
    Ok(RegionData(chunks))
  }
}
#[derive(Debug)]
struct ReadableChunk {
  sections: Vec<Option<paletted::Section>>,
  version:  BlockVersion,
}
impl MessageRead<'_> for ReadableChunk {
  fn read(r: &mut MessageReader) -> Result<Self, ReadError> { r.read_struct() }
}
impl StructRead<'_> for ReadableChunk {
  fn read_struct(mut r: StructReader) -> Result<Self, ReadError> {
    Ok(ReadableChunk {
      sections: r.must_read(0)?,
      version:  BlockVersion::from_index(r.must_read(1)?),
    })
  }
}

impl ReadableChunk {
  pub fn update_chunk(self, chunk: &mut CountedChunk) {
    let mut lock = chunk.lock();
    for (y, section) in self.sections.into_iter().enumerate() {
      if let Some(s) = section {
        let (old_palette, data) = s.into_palette_data();
        if self.version == BlockVersion::latest() {
          lock.inner_mut().section_mut(y as u32).set_from(old_palette, data);
        } else {
          let mut new_palette = Vec::with_capacity(old_palette.len());
          for id in old_palette {
            new_palette.push(lock.wm().block_converter().to_old(id, self.version));
          }
          lock.inner_mut().section_mut(y as u32).set_from(new_palette, data);
        }
      } else {
        lock.inner_mut().clear_section(y as u32);
      }
    }
  }
}

struct WriteableChunk<'a>(&'a CountedChunk);
impl MessageWrite for WriteableChunk<'_> {
  fn write<W: Write>(&self, w: &mut MessageWriter<W>) -> Result<(), WriteError> {
    // TODO: Write light
    w.write_struct(2, |w| {
      w.write_list(self.0.chunk.lock().inner().sections())?;
      w.write_u32(BlockVersion::latest().to_index())?;
      Ok(())
    })
  }
}
