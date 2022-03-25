use super::{PluginEvent, PluginImpl, ServerEvent};
use crate::world::WorldManager;
use crossbeam_channel::{Receiver, Sender};
use mio::{
  net::{UnixListener, UnixStream},
  Events, Interest, Poll, Token, Waker,
};
use std::{
  collections::HashMap,
  fs, io,
  io::{BufRead, BufReader},
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::Arc,
};

pub struct SocketManager {
  wm:       Arc<WorldManager>,
  sockets:  HashMap<Token, UnixStream>,
  waker:    Arc<Waker>,
  next_tok: usize,
  poll:     Poll,
  tok_tx:   Sender<Token>,
  tok_rx:   Receiver<Token>,
  serv_rx:  HashMap<Token, Receiver<ServerEvent>>,
  plug_tx:  HashMap<Token, Sender<PluginEvent>>,
}

pub struct SocketPlugin {
  wm:      Arc<WorldManager>,
  tok:     Token,
  waker:   Arc<Waker>,
  tok_tx:  Sender<Token>,
  serv_tx: Sender<ServerEvent>,
  rx:      Receiver<PluginEvent>,
}

impl SocketManager {
  pub fn new(wm: Arc<WorldManager>) -> SocketManager {
    let poll = Poll::new().unwrap();
    let waker = Waker::new(poll.registry(), Token(0)).unwrap();
    let (tok_tx, tok_rx) = crossbeam_channel::bounded(128);
    SocketManager {
      wm,
      sockets: HashMap::new(),
      waker: Arc::new(waker),
      next_tok: 1,
      poll,
      tok_rx,
      tok_tx,
      serv_rx: HashMap::new(),
      plug_tx: HashMap::new(),
    }
  }

  pub fn add(&mut self, name: String, path: PathBuf) -> Option<SocketPlugin> {
    let mut socket = open(name, path)?;

    let tok = Token(self.next_tok);
    self.next_tok += 1;

    let (serv_tx, serv_rx) = crossbeam_channel::bounded(128);
    let (plug_tx, plug_rx) = crossbeam_channel::bounded(128);
    let plugin = SocketPlugin {
      wm: self.wm.clone(),
      tok,
      waker: self.waker.clone(),
      tok_tx: self.tok_tx.clone(),
      serv_tx,
      rx: plug_rx,
    };
    self
      .poll
      .registry()
      .register(&mut socket, tok, Interest::READABLE | Interest::WRITABLE)
      .unwrap();
    self.sockets.insert(tok, socket);
    self.serv_rx.insert(tok, serv_rx);
    self.plug_tx.insert(tok, plug_tx);
    Some(plugin)
  }

  pub fn listen(mut self) {
    let mut events = Events::with_capacity(1024);
    loop {
      self.poll.poll(&mut events, None).unwrap();
      for ev in &events {
        /*
        let mut sock = self.socket.lock();
        let mut buf = vec![];
        sock.read_until(b'\0', &mut buf).unwrap();
        drop(sock);
        buf.pop(); // Remove null byte
        match serde_json::from_slice(&buf) {
          Ok(v) => Ok(v),
          Err(e) => {
            error!("invalid event from plugin `{}`: {}", "", e);
            Err(())
          }
        }

        let mut sock = self.socket.lock();
        let mut data = serde_json::to_vec(&ev).unwrap();
        data.push(b'\0');
        sock.get_mut().write_all(&data)?;
        sock.get_mut().flush()?;
        Ok(())
        */
      }
    }
  }
}

fn open(name: String, path: PathBuf) -> Option<UnixStream> {
  let sock_path = path.join("server.sock");
  if sock_path.exists() {
    fs::remove_file(&sock_path).unwrap();
  }
  let listener = UnixListener::bind(&sock_path).unwrap();

  start_plugin(name.clone(), &path);

  match listener.accept() {
    Ok((socket, _)) => {
      info!("plugin `{name}` has connected");
      Some(socket)
    }
    Err(e) => {
      error!("accept function failed: {:?}", e);
      None
    }
  }
}

impl SocketPlugin {
  pub fn spawn_listener(self: Arc<Self>) {
    let p = self.clone();
    std::thread::spawn(move || loop {
      match p.read() {
        Ok(ev) => {
          info!("handling event {ev:?}");
          p.handle_event(ev)
        }
        Err(_) => break,
      }
    });
  }

  pub fn read(&self) -> Result<PluginEvent, ()> { Ok(self.rx.recv().unwrap()) }
  pub fn handle_event(&self, e: PluginEvent) {
    match e {
      PluginEvent::Ready => {}
      PluginEvent::Register { ty } => todo!(),
      PluginEvent::SendChat { text } => {
        self.wm.broadcast(text);
      }
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
  pub fn send(&self, ev: ServerEvent) -> io::Result<()> {
    self.tok_tx.send(self.tok);
    self.serv_tx.send(ev);
    self.waker.wake();
    Ok(())
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

impl PluginImpl for Arc<SocketPlugin> {
  fn call(&self, ev: ServerEvent) -> Result<(), ()> {
    match self.send(ev) {
      Ok(_) => Ok(()),
      Err(e) => {
        error!("could not send message to plugin: {e}");
        Err(())
      }
    }
  }
}
