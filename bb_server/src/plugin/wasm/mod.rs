mod ffi;
mod funcs;
mod input;
mod output;

pub use ffi::{FromFfi, ToFfi};
pub use funcs::Env;

use super::{CallError, GlobalServerEvent, PluginImpl, PluginReply, ServerEvent, ServerRequest};
use crate::{
  player::Player,
  world::{MultiChunk, WorldManager},
};
use bb_ffi::CUUID;
use parking_lot::Mutex;
use std::{fs, io, path::Path, process::Command, sync::Arc};
use thiserror::Error;
use wasmer::{Instance, Memory, Module, NativeFunc, Store, WasmPtr, WasmTypeList};

pub struct Plugin {
  inst_mem_lock: Mutex<()>,
  inst:          Instance,
}

/// This is the last argument to every wasm function. It is the pointer type (we
/// are using 32 bit), and it is used for the function to write it's result
/// into.
type OUT = u32;

trait Input {
  type WasmArgs: WasmTypeList;
  /// Calls native, and passes the pointer as the last argument.
  fn call_native<Rets: WasmTypeList>(
    &self,
    native: &NativeFunc<Self::WasmArgs, Rets>,
  ) -> Result<Rets, wasmer::RuntimeError>;
}
trait Output {
  /// Returns the size in pointers. This keeps everything aligned, and means we
  /// use `MemoryView<i32>` everywhere.
  fn size() -> u32;
  /// Returns `Self`, given the address in pointers.
  fn from_addr(mem: &Memory, addr: OUT) -> Self;
}

#[derive(Error, Debug)]
pub enum PluginCreateError {
  #[error("failed to compile: {0}")]
  CompileMissing(io::Error),
  #[error("failed to compile: {0}")]
  CompileFailed(String),
  #[error("couldn't find source: {0}")]
  Missing(io::Error),
  #[error("could not instantiate plugin: {0}")]
  InstantiationError(#[from] wasmer::InstantiationError),
  #[error("could not compile plugin: {0}")]
  CompileError(#[from] wasmer::CompileError),
}

impl Plugin {
  pub fn new(
    name: String,
    path: &Path,
    compile: String,
    output: String,
    wm: Arc<WorldManager>,
  ) -> Result<Self, PluginCreateError> {
    if !compile.is_empty() {
      info!("compiling {name}...");
      let out = Command::new("sh")
        .arg("-c")
        .arg(&compile)
        .current_dir(path)
        .output()
        .map_err(PluginCreateError::CompileMissing)?;
      if out.status.success() {
        info!("compiled {name}:\n{}", String::from_utf8_lossy(&out.stderr));
      } else {
        return Err(PluginCreateError::CompileFailed(String::from_utf8_lossy(&out.stderr).into()));
      }
    }
    let store = Store::default();
    let module =
      Module::new(&store, fs::read(path.join(output)).map_err(PluginCreateError::Missing)?)?;
    let import_object = funcs::imports(&store, wm, name);
    let inst = Instance::new(&module, &import_object)?;
    let plug = Plugin { inst_mem_lock: Mutex::new(()), inst };
    plug.call("init", ()).unwrap();
    Ok(plug)
  }

  fn call_bool<I: Input>(&self, name: &str, input: I) -> Result<bool, CallError> {
    // Try to get function with int. If this fails, error.
    // If the function doesn't exist, we error.
    match self.inst.exports.get_native_function::<I::WasmArgs, u8>(name) {
      Ok(func) => Ok(input.call_native(&func).unwrap() != 0),
      Err(e) => Err(CallError::no_keep(e)),
    }
  }
  fn call_int<I: Input>(&self, name: &str, input: I) -> Result<i32, CallError> {
    // Try to get function with int. If this fails, error.
    // If the function doesn't exist, we error.
    match self.inst.exports.get_native_function::<I::WasmArgs, i32>(name) {
      Ok(func) => Ok(input.call_native(&func).unwrap()),
      Err(e) => Err(CallError::no_keep(e)),
    }
  }
  fn call<I: Input>(&self, name: &str, input: I) -> Result<(), CallError> {
    // Try to get function with int. If this fails, error.
    // If the function doesn't exist, we error.
    match self.inst.exports.get_native_function::<I::WasmArgs, ()>(name) {
      Ok(func) => Ok(input.call_native(&func).unwrap_or(())),
      Err(e) => Err(CallError::no_keep(e)),
    }
  }

  fn malloc_str(&self, text: &str) -> Result<WasmPtr<u8>, CallError> {
    let ptr = self.call_int("wasm_malloc", (text.len() as i32 + 1, 1))? as u32;
    let mem = self.inst.exports.get_memory("memory").unwrap();
    unsafe {
      let _guard = self.inst_mem_lock.lock();
      let view = mem.data_unchecked_mut();
      view[ptr as usize..ptr as usize + text.len()].copy_from_slice(text.as_bytes());
      view[ptr as usize + text.len()] = 0; // Write the nul byte
    }
    Ok(WasmPtr::new(ptr))
  }
  fn free_str(&self, ptr: WasmPtr<u8>, text: &str) -> Result<(), CallError> {
    self.call("wasm_free", (ptr, text.len() as i32 + 1, 1))
  }

  fn generate_chunk(
    &self,
    generator: &str,
    chunk: Arc<Mutex<MultiChunk>>,
    pos: bb_common::math::ChunkPos,
  ) -> Result<(), CallError> {
    let generator_ptr = self.malloc_str(generator)?;
    let ptr = self.call_int("generate_chunk_and_lock", (generator_ptr, pos.x(), pos.z()))?;
    self.free_str(generator_ptr, generator)?;
    if ptr == 0 {
      return Ok(());
    }
    let mem = self.inst.exports.get_memory("memory").unwrap();
    // SAFETY: The plugin has locked the generated chunk buffer, so we can read from
    // it. Accessing this memory is safe for the plugin until we call
    // `unlock_generated_chunk`. In order for the server to not get UB for a
    // malicious plugin, we also lock `inst_mem_lock` for the duration that we have
    // a reference to `data_unchecked`.
    unsafe {
      let _guard = self.inst_mem_lock.lock();
      let view = mem.data_unchecked();
      let chunk_data = &view[ptr as usize..];
      let mut reader = bb_transfer::MessageReader::new(&chunk_data);
      match reader.read::<Vec<bb_common::chunk::paletted::Section>>() {
        Ok(sections) => {
          let mut chunk = chunk.lock();
          for (y, section) in sections.into_iter().enumerate() {
            *chunk.inner_mut().section_mut(y as u32) = section;
          }
        }
        Err(e) => error!("bad chunk: {e}"),
      }
    }
    self.call("unlock_generated_chunk", ())?;

    Ok(())
  }
}

impl PluginImpl for Plugin {
  fn call(&self, _player: Arc<Player>, ev: ServerEvent) -> Result<(), CallError> {
    match ev {
      _ => warn!("todo: ev {ev:?}"),
    }
    Ok(())
  }
  fn call_global(&self, ev: GlobalServerEvent) -> Result<(), CallError> {
    match ev {
      GlobalServerEvent::Tick => self.call("tick", ())?,
      GlobalServerEvent::GenerateChunk { generator, chunk, pos } => {
        self.generate_chunk(&generator, chunk, pos)?
      }
    }
    Ok(())
  }

  fn req(&self, player: Arc<Player>, request: ServerRequest) -> Result<PluginReply, CallError> {
    Ok(PluginReply::Cancel {
      allow: match request {
        ServerRequest::BlockPlace { pos, .. } => self.call_bool(
          "on_block_place",
          (
            CUUID {
              bytes: [
                u32::from_ne_bytes(player.id().as_le_bytes()[0..4].try_into().unwrap()),
                u32::from_ne_bytes(player.id().as_le_bytes()[4..8].try_into().unwrap()),
                u32::from_ne_bytes(player.id().as_le_bytes()[8..12].try_into().unwrap()),
                u32::from_ne_bytes(player.id().as_le_bytes()[12..16].try_into().unwrap()),
              ],
            },
            pos.x(),
            pos.y(),
            pos.z(),
          ),
        )?,
        _ => true,
      },
    })
  }
}
