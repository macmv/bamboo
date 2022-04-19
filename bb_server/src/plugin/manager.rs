/// A struct that manages all plugins. This will handle re-loading all the
/// source files on `/reload`, and will also send events to all the plugins when
/// needed.
pub struct PluginManager {
  pub(super) plugins: Mutex<Vec<Plugin>>,
}

#[cfg(feature = "panda_plugins")]
use super::PandaPlugin;

use super::{GlobalServerEvent, Plugin, ServerEvent, ServerMessage, ServerRequest};
use crate::{block, player::Player, world::WorldManager};
use bb_common::{config::Config, math::Pos, net::sb::ClickWindow, util::Chat};
use parking_lot::Mutex;
use std::{fs, sync::Arc};

impl PluginManager {
  /// Creates a new plugin manager. This will initialize the Ruby interpreter,
  /// and load all plugins from disk. Do not call this multiple times.
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self { PluginManager { plugins: Mutex::new(vec![]) } }

  /// Returns true if plugins should print error messages with colors.
  pub fn use_color(&self) -> bool { true }

  /// Loads all plugins from disk. Call this to reload all plugins.
  pub fn load(&self, wm: Arc<WorldManager>) {
    let mut plugins = self.plugins.lock();
    plugins.clear();

    #[cfg(feature = "socket_plugins")]
    let mut sockets = SocketManager::new(wm.clone());

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
          path.join("plugin.yml").to_str().unwrap(),
          path.join("plugin-default.yml").to_str().unwrap(),
          include_str!("plugin.yml"),
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
                plugins.push(Plugin::new(config, plugin));
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
              plugins.push(Plugin::new(config, python::Plugin::new(name.clone())));
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
              match wasm::Plugin::new(
                name.clone(),
                &path,
                config.get::<_, String>("wasm.compile"),
                config.get::<_, String>("wasm.output"),
                wm.clone(),
              ) {
                Ok(p) => plugins.push(Plugin::new(config, p)),
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
                plugins.push(Plugin::new(config, p));
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

  fn message(&self, msg: ServerMessage) {
    self.plugins.lock().retain(|p| p.call(msg.clone()).is_ok());
  }
  fn message_bool(&self, msg: ServerMessage) -> bool {
    let mut allow = true;
    self.plugins.lock().retain(|p| {
      if let Ok(res) = p.call(msg.clone()) {
        if !res {
          allow = false;
        }
        true
      } else {
        // Remove this plugin
        false
      }
    });
    allow
  }
  fn event(&self, player: Arc<Player>, event: ServerEvent) {
    self.message(ServerMessage::Event { player, event });
  }
  fn req(&self, player: Arc<Player>, request: ServerRequest) -> bool {
    self.message_bool(ServerMessage::Request { reply_id: 0, player, request })
  }
  fn global_event(&self, event: GlobalServerEvent) {
    self.message(ServerMessage::GlobalEvent { event });
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
