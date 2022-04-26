#[cfg(feature = "panda_plugins")]
use super::PandaPlugin;

use super::{GlobalServerEvent, Plugin, ServerEvent, ServerRequest};
use crate::{block, player::Player, world::WorldManager};
use bb_common::{config::Config, math::Pos, net::sb::ClickWindow, util::Chat};
use crossbeam_channel::Select;
use parking_lot::Mutex;
use std::{
  fs,
  sync::Arc,
  time::{Duration, Instant},
};

/// A struct that manages all plugins. This will handle re-loading all the
/// source files on `/reload`, and will also send events to all the plugins when
/// needed.
pub struct PluginManager {
  start:              Instant,
  pub(super) plugins: Mutex<Vec<Plugin>>,
}

impl PluginManager {
  /// Creates a new plugin manager. This will initialize the Ruby interpreter,
  /// and load all plugins from disk. Do not call this multiple times.
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { PluginManager { start: Instant::now(), plugins: Mutex::new(vec![]) } }

  /// Returns true if plugins should print error messages with colors.
  pub fn use_color(&self) -> bool { true }

  /// Loads all plugins from disk. Call this to reload all plugins.
  pub fn load(&self, wm: Arc<WorldManager>) {
    let mut plugins = self.plugins.lock();
    plugins.clear();

    #[cfg(feature = "socket_plugins")]
    let mut sockets = super::socket::SocketManager::new(wm.clone());

    let iter = match fs::read_dir("plugins") {
      Ok(v) => v,
      Err(e) => {
        warn!("error reading directory `plugins`: {e}");
        return;
      }
    };
    for f in iter {
      let f = f.unwrap();
      let m = fs::metadata(f.path()).unwrap();
      if m.is_dir() {
        let path = f.path();
        let config = Config::new(
          path.join("plugin.toml").to_str().unwrap(),
          path.join("plugin-default.toml").to_str().unwrap(),
          include_str!("plugin.toml"),
        );
        if !config.get::<_, bool>("enabled") {
          continue;
        }
        let ty: String = config.get("type");
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        match ty.as_str() {
          "socket" => {
            info!("found socket plugin at {}", path.to_str().unwrap());
            #[cfg(feature = "socket_plugins")]
            {
              if let Some(plugin) = sockets.add(name.clone(), f.path()) {
                plugins.push(Plugin::new(name.clone(), config, plugin));
              }
            }
            #[cfg(not(feature = "socket_plugins"))]
            {
              info!("socket plugins are disabling, skipping {}", path.to_str().unwrap());
            }
          }
          "python" => {
            info!("found python plugin at {}", path.to_str().unwrap());
            #[cfg(feature = "python_plugins")]
            {
              plugins.push(Plugin::new(
                name.clone(),
                config,
                super::python::Plugin::new(name.clone()),
              ));
            }
            #[cfg(not(feature = "python_plugins"))]
            {
              info!("python plugins are disabling, skipping {}", path.to_str().unwrap());
            }
          }
          "wasm" => {
            info!("found wasm plugin at {}", path.to_str().unwrap());
            #[cfg(feature = "wasm_plugins")]
            {
              match super::wasm::Plugin::new(
                name.clone(),
                &path,
                config.get::<_, String>("wasm.compile"),
                config.get::<_, String>("wasm.output"),
                wm.clone(),
              ) {
                Ok(p) => plugins.push(Plugin::new(name.clone(), config, p)),
                Err(e) => error!("error loading {name}: {e}"),
              }
            }
            #[cfg(not(feature = "wasm_plugins"))]
            {
              info!("wasm plugins are disabling, skipping {}", path.to_str().unwrap());
            }
          }
          "panda" => {
            let main_path = f.path().join("main.pand");
            info!("found panda plugin at {}", main_path.to_str().unwrap());
            #[cfg(feature = "panda_plugins")]
            {
              if main_path.exists() && main_path.is_file() {
                let name = f.path().file_stem().unwrap().to_str().unwrap().to_string();
                let mut p = PandaPlugin::new(plugins.len(), name.clone(), wm.clone());

                p.load_from_dir(&f.path(), self);
                p.call_init();
                plugins.push(Plugin::new(name.clone(), config, p));
              } else {
                error!("plugin `{name}` does not have a `main.pand` file");
              }
            }
            #[cfg(not(feature = "panda_plugins"))]
            {
              info!("panda plugins are disabling, skipping {}", main_path.to_str().unwrap());
            }
          }
          _ => error!("plugin `{name}` has invalid plugin type: `{ty}`"),
        }
      }
    }

    #[cfg(feature = "socket_plugins")]
    {
      let plugins = sockets.take_plugins();
      std::thread::spawn(|| {
        sockets.listen();
      });
      for plug in plugins {
        plug.wait_for_ready().unwrap();
        plug.clone().spawn_listener();
      }
    }
  }

  fn event(&self, player: Arc<Player>, event: ServerEvent) {
    self.plugins.lock().retain(|p| match p.call(player.clone(), event.clone()) {
      Ok(_) => true,
      Err(e) => e.keep,
    });
  }
  fn global_event(&self, event: GlobalServerEvent) {
    self.plugins.lock().retain(|p| match p.call_global(event.clone()) {
      Ok(_) => true,
      Err(e) => e.keep,
    });
  }
  fn req(&self, player: Arc<Player>, request: ServerRequest) -> bool {
    let reply_id = self.start.elapsed().as_micros() as u32;
    let mut plugins = self.plugins.lock();
    // Send all the events first.
    plugins.retain(|p| match p.req(reply_id, player.clone(), request.clone()) {
      Ok(_) => true,
      Err(e) => e.keep,
    });
    // Then wait on all of them.
    let mut allow = true;
    let mut plugins_left: Vec<_> = plugins.iter_mut().collect();
    let deadline = Instant::now() + Duration::from_millis(50);
    while !plugins_left.is_empty() {
      let mut sel = Select::new();
      for p in &plugins_left {
        sel.recv(p.rx());
      }
      let index;
      let message;
      let plugin;
      match sel.select_deadline(deadline) {
        Ok(op) => {
          index = op.index();
          let plugin = &plugins_left[index];
          message = op.recv(plugin.rx()).unwrap();
        }
        Err(_) => return allow,
      }
      plugin = &mut plugins_left[index];
      let now = self.start.elapsed().as_micros() as u32;
      match plugin.check_allow(message, now, reply_id) {
        Some(false) => allow = false,
        Some(true) => {}
        None => continue,
      }
      plugins_left.remove(index);
    }

    allow
  }
  pub fn on_tick(&self) { self.global_event(GlobalServerEvent::Tick); }
  pub fn on_block_place(&self, player: Arc<Player>, pos: Pos, block: block::Type) -> bool {
    self.req(player, ServerRequest::BlockPlace { pos, block })
  }
  pub fn on_block_break(&self, player: Arc<Player>, pos: Pos, block: block::Type) -> bool {
    self.req(player, ServerRequest::BlockBreak { pos, block })
  }
  pub fn on_chat_message(&self, player: Arc<Player>, message: Chat) {
    self.event(player, ServerEvent::Chat { text: message.to_plain() });
  }
  pub fn on_player_join(&self, player: Arc<Player>) {
    self.event(player, ServerEvent::PlayerJoin {});
  }
  pub fn on_player_leave(&self, player: Arc<Player>) {
    self.event(player, ServerEvent::PlayerLeave {});
  }
  pub fn on_click_window(&self, player: Arc<Player>, slot: i32, mode: ClickWindow) -> bool {
    self.req(player, ServerRequest::ClickWindow { slot, mode })
  }
}
