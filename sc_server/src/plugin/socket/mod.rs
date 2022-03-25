use super::{PluginEvent, PluginImpl, ServerEvent};
use crate::world::WorldManager;
use crossbeam_channel::{Receiver, Sender};
use mio::{event::Event, net::UnixStream, Events, Interest, Poll, Token, Waker};
use std::{
  collections::HashMap,
  fs, io,
  io::{BufRead, BufReader, Read, Write},
  os::unix::net::UnixListener as StdUnixListener,
  path::{Path, PathBuf},
  process::{Command, Stdio},
  sync::Arc,
};

struct WrappedSocket {
  stream:   UnixStream,
  outgoing: Vec<u8>,
  incoming: Vec<u8>,
  plug_tx:  Sender<PluginEvent>,
}

pub struct SocketManager {
  wm:       Arc<WorldManager>,
  sockets:  HashMap<Token, WrappedSocket>,
  waker:    Arc<Waker>,
  next_tok: usize,
  poll:     Poll,
  tok_tx:   Sender<Token>,
  tok_rx:   Receiver<Token>,
  serv_rx:  HashMap<Token, Receiver<ServerEvent>>,
  plugins:  Vec<Arc<SocketPlugin>>,
}

pub struct SocketPlugin {
  wm:      Arc<WorldManager>,
  tok:     Token,
  waker:   Arc<Waker>,
  tok_tx:  Sender<Token>,
  serv_tx: Sender<ServerEvent>,
  rx:      Receiver<PluginEvent>,
}

const LISTEN: Token = Token(0);

impl SocketManager {
  pub fn new(wm: Arc<WorldManager>) -> SocketManager {
    let poll = Poll::new().unwrap();
    let waker = Waker::new(poll.registry(), LISTEN).unwrap();
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
      plugins: vec![],
    }
  }

  pub fn add(&mut self, name: String, path: PathBuf) -> Option<Arc<SocketPlugin>> {
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
    self.sockets.insert(tok, WrappedSocket::new(socket, plug_tx));
    self.serv_rx.insert(tok, serv_rx);
    let plugin = Arc::new(plugin);
    self.plugins.push(plugin.clone());
    Some(plugin)
  }

  pub fn take_plugins(&mut self) -> Vec<Arc<SocketPlugin>> {
    std::mem::replace(&mut self.plugins, vec![])
  }

  pub fn listen(mut self) {
    let mut events = Events::with_capacity(1024);
    loop {
      self.poll.poll(&mut events, None).unwrap();
      for ev in &events {
        match ev.token() {
          LISTEN => loop {
            // Someone wants to send something to a plugin
            let plugin_tok = match self.tok_rx.try_recv() {
              Ok(tok) => tok,
              Err(_) => break,
            };
            // Don't want to block here
            let msg = self.serv_rx[&plugin_tok].try_recv().unwrap();

            match self.sockets.get_mut(&plugin_tok).unwrap().send(msg) {
              Ok(_) => {}
              Err(e) => error!("error sending to plugin: {e}"),
            }
          },
          token => {
            // One of our sockets has just changed state
            self.handle_socket_change(ev, token);
          }
        }
        /*

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

  fn handle_socket_change(&mut self, ev: &Event, tok: Token) -> io::Result<()> {
    let socket = self.sockets.get_mut(&tok).unwrap();
    if ev.is_readable() {
      socket.try_read()?;
    }
    if ev.is_writable() {
      socket.try_flush()?;
    }
    Ok(())
  }
}

impl WrappedSocket {
  pub fn new(stream: UnixStream, plug_tx: Sender<PluginEvent>) -> Self {
    WrappedSocket {
      stream,
      outgoing: Vec::with_capacity(1024),
      incoming: Vec::with_capacity(1024),
      plug_tx,
    }
  }

  pub fn send(&mut self, ev: ServerEvent) -> io::Result<()> {
    self.outgoing.append(&mut serde_json::to_vec(&ev).unwrap());
    self.outgoing.push(b'\0');
    self.try_flush()?;
    Ok(())
  }

  pub fn try_read(&mut self) -> io::Result<()> {
    loop {
      let mut buf = vec![0; 1024];
      match self.stream.read(&mut buf) {
        Ok(n) => {
          self.incoming.extend(buf);
          self.read_events()?;
        }
        Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock) => return Ok(()),
        Err(e) => return Err(e),
      }
    }
  }

  fn read_events(&mut self) -> io::Result<()> {
    while let Some(idx) = self.incoming.iter().position(|&x| x == b'\0') {
      let buf = &self.incoming[..idx];

      let res = serde_json::from_slice(&buf);
      self.incoming.drain(..idx + 1);
      match res {
        Ok(v) => {
          self.plug_tx.send(v).unwrap();
        }
        Err(e) => {
          error!("invalid event from plugin `{}`: {}", "", e);
          return Ok(());
        }
      }
    }
    Ok(())
  }

  pub fn try_flush(&mut self) -> io::Result<()> {
    match self.stream.write(&self.outgoing) {
      Ok(n) => {
        self.outgoing.drain(..n);
        Ok(())
      }
      Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock) => Ok(()),
      Err(e) => return Err(e),
    }
  }
}

fn open(name: String, path: PathBuf) -> Option<UnixStream> {
  let sock_path = path.join("server.sock");
  if sock_path.exists() {
    fs::remove_file(&sock_path).unwrap();
  }
  let listener = StdUnixListener::bind(&sock_path).unwrap();

  start_plugin(name.clone(), &path);

  match listener.accept() {
    Ok((socket, _)) => {
      info!("plugin `{name}` has connected");
      socket.set_nonblocking(true).unwrap();
      Some(UnixStream::from_std(socket))
    }
    Err(e) => {
      error!("accept function failed: {:?}", e);
      None
    }
  }
}

impl SocketPlugin {
  pub fn spawn_listener(self: Arc<Self>) {
    info!("spawning listener");
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
    info!("spawned listener");
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
    info!("waiting for ready");
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
    self.tok_tx.send(self.tok).unwrap();
    self.serv_tx.send(ev).unwrap();
    self.waker.wake().unwrap();
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
