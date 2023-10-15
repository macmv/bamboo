//! Loads vanilla region files and Bamboo region files from disk. The former
//! is made for vanilla compatability, and the latter is a custom protobuf
//! format which is easier to maintain.
//!
//! Not to be confused with `bbr` (bamboo region), which is for a custom world
//! format.

use crate::block;
use bb_common::{
  chunk::Section,
  math::{ChunkPos, SectionRelPos},
  nbt::{Tag, WrongTag, NBT},
  util::ThreadPool,
};
use std::{fmt, fs, io, path::Path, str::FromStr, sync::Arc};

use super::World;

pub enum RegionError {
  IO(io::Error),
  WrongTag(WrongTag),
}

impl From<io::Error> for RegionError {
  fn from(e: io::Error) -> Self { RegionError::IO(e) }
}
impl From<WrongTag> for RegionError {
  fn from(e: WrongTag) -> Self { RegionError::WrongTag(e) }
}

impl fmt::Display for RegionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::IO(e) => write!(f, "{e}"),
      Self::WrongTag(e) => write!(f, "{e}"),
    }
  }
}

fn parse_region_name(name: &str) -> Option<(i32, i32)> {
  let mut sections = name.split('.');
  if sections.next()? != "r" {
    return None;
  }
  let x = sections.next()?.parse().ok()?;
  let z = sections.next()?.parse().ok()?;
  if sections.next()? != "mca" {
    return None;
  }
  if sections.next().is_some() {
    return None;
  }
  Some((x, z))
}

impl World {
  pub fn load_from_disk(self: &Arc<Self>, path: &Path) -> io::Result<()> {
    let chunks = path.join("region");
    let pool = ThreadPool::auto("vanilla regions", || ());
    for f in fs::read_dir(chunks)? {
      let f = f?;
      if f.metadata()?.is_file() {
        let path = f.path();
        let name = match path.file_name().unwrap().to_str() {
          Some(s) => s,
          None => continue,
        };
        let (_x, _z) = match parse_region_name(name) {
          Some(v) => v,
          None => continue,
        };
        let w = self.clone();
        pool.execute(move |_| match w.load_region_file(&path) {
          Ok(_) => {}
          Err(e) => error!("invalid vanilla region file at {}: {}", path.display(), e),
        });
      }
    }
    pool.wait();
    Ok(())
  }

  fn load_region_file(&self, path: &Path) -> Result<(), RegionError> {
    let data = fs::read(path)?;
    let header = &data[..8192];
    // `offset` is an offset into the file, not an offset into the chunks table.
    let chunks = &data;
    for id in 0..1024 {
      let start = id * 4;
      let num = u32::from_be_bytes(header[start..start + 4].try_into().unwrap());
      let offset: usize = ((num >> 8) & 0xffffff) as usize * 4096;
      let mut size: usize = (num & 0xff) as usize * 4096;
      if size == 0 {
        continue;
      }

      if offset + size > chunks.len() {
        size = chunks.len() - offset;
      } else if offset >= chunks.len() {
        error!("section had invalid index: {offset:#x} size {size:#x} (len: {:#x})", chunks.len());
        continue;
      }

      let chunk = &chunks[offset..offset + size];
      let header = &chunk[..5];
      let len = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
      let _compression = header[4];
      let nbt = NBT::deserialize_file(chunk[5..5 + len - 1].to_vec()).unwrap().into_tag();

      // 1.8 uses capitalized names
      // 1.12.2 uses lowercase names.
      // 1.18 uses a mix of both (wtf???)
      let is_capital_names;
      let nbt = if nbt.compound()?.contains_key("Level") {
        is_capital_names = true;
        &nbt.compound()?["Level"]
      } else {
        is_capital_names = false;
        &nbt
      };
      let level = nbt.compound()?;
      let sections_key = if is_capital_names { "Sections" } else { "sections" };
      if !level.contains_key(sections_key) {
        continue;
      }

      // the chunk_x and chunk_z values are absolute.
      let chunk_x = level["xPos"].int()?;
      let chunk_z = level["zPos"].int()?;
      let pos = ChunkPos::new(chunk_x, chunk_z);

      // TODO: Light updates!
      self.chunk(pos, |mut chunk| {
        for s in level[sections_key].list()? {
          let section = s.compound()?;
          let y = section["Y"].byte()?;
          if y < 0 {
            // TODO: Handle negative chunks
            continue;
          }
          let y = y as u32;
          if is_capital_names {
            if section.contains_key("Blocks") {
              // is 1.8
              let blocks = section["Blocks"].byte_arr()?;
              let data = section["Data"].byte_arr()?;
              let section = chunk.inner_mut().section_mut(y);
              for y in 0..16 {
                for z in 0..16 {
                  for x in 0..16 {
                    let index = ((y as usize * 16) + z as usize) * 16 + x as usize;
                    let mask = 0x0f << ((x % 2) * 4);
                    let id = (blocks[index] as u32) << 4 | (data[index / 2] & mask) as u32;
                    let old =
                      self.block_converter().to_latest(id, bb_common::version::BlockVersion::V1_8);
                    section.set_block(SectionRelPos::new(x, y, z), old);
                  }
                }
              }
            } else {
              // is 1.12+
              // Skip air sections
              if !section.contains_key("BlockStates") {
                continue;
              }
              let block_states =
                section["BlockStates"].long_arr()?.iter().map(|v| *v as u64).collect();
              let palette: Vec<_> = section["Palette"]
                .list()?
                .iter()
                .map(|it| parse_state(self.block_converter(), it))
                .collect::<Result<_, _>>()?;
              let section = chunk.inner_mut().section_mut(y);
              section.set_from(palette, block_states);
            }
          } else {
            // is 1.12+
            let block_states = section["block_states"].compound()?;
            let palette: Vec<_> = block_states["palette"]
              .list()?
              .iter()
              .map(|it| parse_state(self.block_converter(), it))
              .collect::<Result<_, _>>()?;
            // The section will be full of one type
            let section = chunk.inner_mut().section_mut(y);
            if !block_states.contains_key("data") {
              assert_eq!(palette.len(), 1);
              section.fill(SectionRelPos::new(0, 0, 0), SectionRelPos::new(15, 15, 15), palette[0]);
            } else {
              let data = block_states["data"].long_arr()?.iter().map(|v| *v as u64).collect();
              section.set_from(palette, data);
            }
          }
        }
        Ok::<(), RegionError>(())
      })?;
    }
    Ok(())
  }
}

fn parse_state(conv: &block::TypeConverter, item: &Tag) -> Result<u32, RegionError> {
  let item = item.compound()?;
  let name = item["Name"].string()?;
  let name = name.strip_prefix("minecraft:").unwrap();
  let mut ty = conv.get(block::Kind::from_str(name).unwrap()).default_type();

  if item.contains_key("Properties") {
    let props = item["Properties"].compound()?;
    for (key, val) in props {
      match val {
        Tag::String(v) if v == "true" => ty.set_prop(key, true),
        Tag::String(v) if v == "false" => ty.set_prop(key, false),
        Tag::String(v) if v.parse::<u32>().is_ok() => ty.set_prop(key, v.parse::<u32>().unwrap()),
        Tag::String(v) => ty.set_prop(key, v.to_uppercase().as_str()),
        _ => unreachable!(),
      }
    }
  }

  Ok(ty.id())
}
