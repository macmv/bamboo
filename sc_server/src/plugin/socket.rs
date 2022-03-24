use parking_lot::Mutex;
use std::{
  fs,
  io::{BufRead, BufReader},
  os::unix::net::{UnixListener, UnixStream},
  path::{Path, PathBuf},
  process::{Command, Stdio},
};

pub struct SocketPlugin {
  name:   String,
  socket: Mutex<BufReader<UnixStream>>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum Event {
  Register { ty: String },
  Ready,
}

impl SocketPlugin {
  pub fn read(&self) -> Event {
    let mut sock = self.socket.lock();
    let mut buf = vec![];
    sock.read_until(b'\0', &mut buf).unwrap();
    drop(sock);
    buf.pop(); // Remove null byte
    loop {
      match serde_json::from_slice(&buf) {
        Ok(v) => return v,
        Err(e) => {
          error!("invalid event from plugin `{}`: {}", self.name, e);
        }
      }
    }
  }
  pub fn handle_event(&self, e: Event) {
    match e {
      Event::Ready => {}
      Event::Register { ty } => todo!(),
    }
  }
  pub fn wait_for_ready(&self) {
    loop {
      match self.read() {
        Event::Ready => break,
        e => self.handle_event(e),
      }
    }
    info!("plugin `{}` is ready", self.name);
  }
}

pub fn open(plugin: String, path: PathBuf) -> Option<SocketPlugin> {
  let sock_path = path.join("server.sock");
  if sock_path.exists() {
    fs::remove_file(&sock_path).unwrap();
  }
  let listener = UnixListener::bind(&sock_path).unwrap();

  start_plugin(plugin.clone(), &path);

  match listener.accept() {
    Ok((socket, _)) => {
      info!("plugin `{plugin}` has connected");
      return Some(SocketPlugin { name: plugin, socket: Mutex::new(BufReader::new(socket)) });
    }
    Err(e) => {
      error!("accept function failed: {:?}", e);
      None
    }
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
