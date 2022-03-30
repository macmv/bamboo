#[macro_use]
extern crate log;

pub mod block;
pub mod command;
pub mod entity;
pub mod item;
pub mod math;
pub mod net;
pub mod player;
pub mod plugin;
pub mod util;
pub mod world;

use crate::{plugin::panda::PandaPlugin, world::WorldManager};
use std::sync::Arc;
use panda::Panda;

pub fn generate_panda_docs() {
  info!("generating sugarlang docs...",);

  let plugin = PandaPlugin::new(0, "".into(), Arc::new(WorldManager::new()));
  let mut pd = Panda::new();
  plugin.add_builtins(&mut pd);
  plugin.generate_docs(&pd);

  info!(
    "generated docs at {}",
    std::env::current_dir().unwrap().join("target/sl_docs/bamboo/index.html").display()
  );
}