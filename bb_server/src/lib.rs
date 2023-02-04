#![allow(clippy::needless_question_mark, clippy::upper_case_acronyms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate thiserror;

use rand::rngs::ThreadRng;
use std::cell::RefCell;

pub mod block;
pub mod command;
pub mod config;
pub mod data;
pub mod enchantment;
pub mod entity;
pub mod event;
pub mod item;
pub mod math;
pub mod net;
pub mod particle;
pub mod player;
pub mod plugin;
pub mod rcon;
pub mod tags;
pub mod util;
pub mod world;

use config::Config;

#[cfg(feature = "panda_plugins")]
pub fn generate_panda_docs() {
  use crate::{plugin::panda::PandaPlugin, world::WorldManager};
  use panda::Panda;
  use std::sync::Arc;

  info!("generating panda docs...",);
  let plugin = PandaPlugin::new(0, "".into(), Arc::new(WorldManager::new(false)));
  let mut pd = Panda::new();
  plugin.add_builtins(&mut pd);
  plugin.generate_docs(&pd);

  info!(
    "generated docs at {}",
    std::env::current_dir().unwrap().join("target/panda_docs/index.html").display()
  );
}

#[cfg(not(feature = "panda_plugins"))]
pub fn generate_panda_docs() {
  info!("panda plugins disabled, cannot generate docs");
}

/// Loads the config at the given path, using the server-provided default
/// config.
pub fn load_config(path: &str) -> Config { bb_common::config::new_at_write_default(path) }
/// Loads the config at the given path, using the server-provided default
/// config. This will then write the default config to the `default` path
/// provided.
pub fn load_config_write_default(path: &str, default: &str) -> Config {
  bb_common::config::new_at_write_default_to(path, default)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Generates docs on test. This is useful for the pipeline, as we can just
  /// compile one binary for coverage results and docs.
  #[test]
  fn generate_docs() { generate_panda_docs(); }
}

thread_local!(pub(crate) static RNG: RefCell<ThreadRng> = RefCell::new(rand::thread_rng()));
