#![allow(clippy::needless_question_mark, clippy::upper_case_acronyms)]

#[macro_use]
extern crate log;

use rand::rngs::ThreadRng;
use std::cell::RefCell;

pub mod block;
pub mod command;
pub mod data;
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

#[cfg(feature = "panda_plugins")]
pub fn generate_panda_docs() {
  use crate::{plugin::panda::PandaPlugin, world::WorldManager};
  use panda::Panda;
  use std::sync::Arc;

  info!("generating panda docs...",);
  let plugin = PandaPlugin::new(0, "".into(), Arc::new(WorldManager::new()));
  let mut pd = Panda::new();
  plugin.add_builtins(&mut pd);
  plugin.generate_docs(&pd);

  info!(
    "generated docs at {}",
    std::env::current_dir().unwrap().join("target/sl_docs/bamboo/index.html").display()
  );
}

#[cfg(not(feature = "panda_plugins"))]
pub fn generate_panda_docs() {
  info!("panda plugins disabled, cannot generate docs");
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
