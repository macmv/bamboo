mod funcs;
mod input;
mod output;

use super::{CallError, GlobalServerEvent, PluginImpl, PluginReply, ServerEvent, ServerRequest};
use crate::{
  player::Player,
  world::{CountedChunk, WorldManager},
};
use bb_ffi::CUUID;
use parking_lot::Mutex;
use std::{fs, io, path::Path, process::Command, sync::Arc};
use thiserror::Error;
use wasmer::{ExportError, Instance, Memory, Module, NativeFunc, Store, WasmTypeList};

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
  ) -> Option<Rets>;
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
    // Try to get function with bool. If this fails, try with no return. If that
    // fails, error out.
    //
    // If the function doesn't exist, we ignore the error.
    match self.inst.exports.get_native_function::<I::WasmArgs, u8>(name) {
      Ok(func) => Ok(input.call_native(&func).unwrap() != 0),
      Err(ExportError::IncompatibleType) => {
        match self.inst.exports.get_native_function::<I::WasmArgs, ()>(name) {
          Ok(func) => {
            input.call_native(&func);
            Ok(true)
          }
          Err(e @ ExportError::IncompatibleType) => Err(CallError::no_keep(e)),
          Err(ExportError::Missing(_)) => Ok(true),
        }
      }
      Err(ExportError::Missing(_)) => Ok(true),
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
    self.call_bool(name, input)?;
    Ok(())
  }

  fn generate_chunk(
    &self,
    generator: &str,
    chunk: Arc<CountedChunk>,
    pos: bb_common::math::ChunkPos,
  ) -> Result<(), CallError> {
    let chunk = chunk.lock();
    for section in chunk.inner().sections().flatten() {
      let chunk_data = section.data().long_array();
      let chunk_ptr = self.call_int("malloc_u64", chunk_data.len() as i32)? as u32;
      if chunk_ptr == 0 || chunk_ptr.checked_add(chunk_data.len() as u32).is_none() {
        // malloc didn't allocate enough
        // TODO: Error out here, and restart the plugin
        continue;
      }
      let mem = self.inst.exports.get_memory("memory").unwrap();
      // SAFETY: We have locked the plugin for memory writing.
      unsafe {
        let _guard = self.inst_mem_lock.lock();
        let view = mem.view::<u64>();
        view.subarray(chunk_ptr, chunk_ptr + chunk_data.len() as u32).copy_from(chunk_data);
        // Make sure we drop `data` before using anything within self.inst
        // again.
      };
      self.call("generate_chunk", (chunk_ptr as i32, chunk_data.len() as i32))?;
      self.call("free_u64", chunk_ptr as i32)?;
    }
    Ok(())
  }
}

impl PluginImpl for Plugin {
  fn call(&self, player: Arc<Player>, ev: ServerEvent) -> Result<(), CallError> {
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
