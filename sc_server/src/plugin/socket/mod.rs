use super::{PluginEvent, PluginImpl, ServerEvent};
use parking_lot::Mutex;
use std::{
  fs,
  io::{BufRead, BufReader, Write},
  os::unix::net::{UnixListener, UnixStream},
  path::{Path, PathBuf},
  process::{Command, Stdio},
};

pub struct SocketPlugin {
  socket: Mutex<BufReader<UnixStream>>,
}

impl SocketPlugin {
  pub fn new(name: String, path: PathBuf) -> Option<SocketPlugin> {
    let sock_path = path.join("server.sock");
    if sock_path.exists() {
      fs::remove_file(&sock_path).unwrap();
    }
    let listener = UnixListener::bind(&sock_path).unwrap();

    start_plugin(name.clone(), &path);

    match listener.accept() {
      Ok((socket, _)) => {
        info!("plugin `{name}` has connected");
        return Some(SocketPlugin { socket: Mutex::new(BufReader::new(socket)) });
      }
      Err(e) => {
        error!("accept function failed: {:?}", e);
        None
      }
    }
  }

  pub fn read(&self) -> Result<PluginEvent, ()> {
    let mut sock = self.socket.lock();
    let mut buf = vec![];
    sock.read_until(b'\0', &mut buf).unwrap();
    drop(sock);
    buf.pop(); // Remove null byte
    loop {
      match serde_json::from_slice(&buf) {
        Ok(v) => return Ok(v),
        Err(e) => {
          error!("invalid event from plugin `{}`: {}", "", e);
          return Err(());
        }
      }
    }
  }
  pub fn handle_event(&self, e: PluginEvent) {
    match e {
      PluginEvent::Ready => {}
      PluginEvent::Register { ty } => todo!(),
    }
  }
  pub fn wait_for_ready(&self) -> Result<(), ()> {
    loop {
      match self.read()? {
        PluginEvent::Ready => break,
        e => self.handle_event(e),
      }
    }
    info!("plugin `{}` is ready", "");
    Ok(())
  }
  pub fn send(&self, ev: ServerEvent) {
    let mut sock = self.socket.lock();
    let mut data = serde_json::to_vec(&ev).unwrap();
    data.push(b'\0');
    sock.get_mut().write_all(&data).unwrap();
    sock.get_mut().flush().unwrap();
  }
}

fn start_plugin(plugin: String, path: &Path) {
  let mut child = match Command::new("./start.sh")
    .current_dir(std::env::current_dir().unwrap().join(path))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
  {
    Ok(child) => child,
    Err(e) => {
      error!("could not start plugin: {e}");
      return;
    }
  };

  let mut stdout = BufReader::new(child.stdout.take().expect("Failed to open stdout"));
  std::thread::spawn(move || {
    let mut line = String::new();
    loop {
      match stdout.read_line(&mut line) {
        Ok(n) => {
          if n == 0 {
            warn!("plugin `{plugin}` has exited");
            break;
          }
          info!("plugin `{plugin}`: {}", line.trim());
          line.clear();
        }
        Err(e) => {
          error!("error reading stdout from plugin `{plugin}`: {e}");
          break;
        }
      }
    }
  });
}

impl PluginImpl for SocketPlugin {
  fn call(&self, ev: ServerEvent) {}
}
