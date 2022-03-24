//! Loads vanilla region files and Sugarcane region files from disk. The former
//! is made for vanilla compatability, and the latter is a custom protobuf
//! format which is easier to maintain.

use crate::block;
use sc_common::{
  chunk::Section,
  math::{ChunkPos, Pos},
  nbt::NBT,
};
use std::{fs, io, path::Path, str::FromStr};

use super::World;

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
  pub fn load_from_disk(&self, path: &Path) -> io::Result<()> {
    let chunks = path.join("region");
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
        self.load_region_file(&path)?;
      }
    }
    Ok(())
  }

  fn load_region_file(&self, path: &Path) -> io::Result<()> {
    let data = fs::read(path)?;
    let header = &data[..8192];
    // `offset` is an offset into the file, not an offset into the chunks table.
    let chunks = &data;
    for id in 0..1024 {
      let start = id * 4;
      let num = u32::from_be_bytes(header[start..start + 4].try_into().unwrap());
      let offset: usize = ((num >> 8) & 0xffffff) as usize * 4096;
      let size: usize = (num & 0xff) as usize * 4096;
      if size == 0 {
        continue;
      }

      let chunk = &chunks[offset..offset + size];
      let header = &chunk[..5];
      let len = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
      let _compression = header[4];
      let nbt = NBT::deserialize_file(chunk[5..5 + len - 1].to_vec()).unwrap().into_tag();

      let is_v8;
      let nbt = if nbt.unwrap_compound().contains_key("Level") {
        is_v8 = true;
        &nbt.unwrap_compound()["Level"]
      } else {
        is_v8 = false;
        &nbt
      };
      let level = nbt.unwrap_compound();
      let sections_key = if is_v8 { "Sections" } else { "sections" };
      if !level.contains_key(sections_key) {
        continue;
      }

      // the chunk_x and chunk_z values are absolute.
      let chunk_x = level["xPos"].unwrap_int();
      let chunk_z = level["zPos"].unwrap_int();
      let pos = ChunkPos::new(chunk_x, chunk_z);

      self.chunk(pos, |mut chunk| {
        for s in level[sections_key].unwrap_list() {
          let section = s.unwrap_compound();
          let y = section["Y"].unwrap_byte();
          if y < 0 {
            // TODO: Handle negative chunks
            continue;
          }
          let y = y as u32;
          if is_v8 {
            let blocks = section["Blocks"].unwrap_byte_arr();
            let data = section["Data"].unwrap_byte_arr();
            let section = chunk.inner_mut().section_mut(y);
            for y in 0..16 {
              for z in 0..16 {
                for x in 0..16 {
                  let index = (((y * 16) + z) * 16 + x) as usize;
                  let mask = 0x0f << (x as u8 % 2) * 4;
                  let id = (blocks[index] as u32) << 4 | (data[index / 2] & mask) as u32;
                  let old =
                    self.block_converter().to_latest(id, sc_common::version::BlockVersion::V1_8);
                  section.set_block(Pos::new(x, y, z), old).unwrap();
                }
              }
            }
          } else {
            let block_states = section["block_states"].unwrap_compound();
            // Skip air sections
            if !block_states.contains_key("data") {
              continue;
            }
            let data = block_states["data"].unwrap_long_arr().iter().map(|v| *v as u64).collect();
            let palette: Vec<_> = block_states["palette"]
              .unwrap_list()
              .iter()
              .map(|item| {
                let item = item.unwrap_compound();
                // TODO: Read properties
                let name = item["Name"].unwrap_string();
                let name = name.strip_prefix("minecraft:").unwrap();
                self.block_converter().get(block::Kind::from_str(name).unwrap()).default_type().id()
              })
              .collect();
            let section = chunk.inner_mut().section_mut(y);
            section.set_from(palette, data);
          }
        }
      });
    }
    Ok(())
  }
}
