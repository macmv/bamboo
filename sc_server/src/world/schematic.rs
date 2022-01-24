use super::CountedChunk;
use sc_common::{
  math::ChunkPos,
  util::nbt::{ParseError, NBT},
};
use std::{collections::HashMap, fs, fs::File, io::Read};

pub fn load_from_file(
  chunks: &mut HashMap<ChunkPos, CountedChunk>,
  path: &str,
) -> Result<(), ParseError> {
  let mut f = File::open(path).expect("no file found");
  let metadata = fs::metadata(path).expect("unable to read metadata");
  let mut buf = vec![0; metadata.len() as usize];
  f.read(&mut buf).expect("file was too large");

  let tag = NBT::deserialize_file(buf);
  dbg!(&tag);
  Ok(())
}
