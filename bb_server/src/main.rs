#[macro_use]
extern crate log;

use bb_common::config::Config;
use bb_server::{net::ConnectionManager, rcon::RCon, world::WorldManager};
use clap::Parser;
use std::{sync::Arc, thread};

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
  /// If set, the server will generate panda docs and exit, without doing
  /// anything else.
  #[clap(long)]
  only_docs: bool,
  /// If set, then docs will not be written. They are written by default so
  /// that new users can easily find them. If this is set, `only_docs` will
  /// be ignored.
  #[clap(long)]
  no_docs:   bool,
}

// #[derive(Clone)]
// pub struct ServerImpl {
//   worlds: Arc<WorldManager>,
// }
//
// #[tonic::async_trait]
// impl Minecraft for ServerImpl {
//   type ConnectionStream = ReceiverStream<Result<Packet, Status>>;
//   async fn connection(
//     &self,
//     req: Request<Streaming<Packet>>,
//   ) -> Result<Response<Self::ConnectionStream>, Status> {
//     let (tx, rx) = mpsc::channel(8);
//
//     // We need to wait for a packet to be recieved from the proxy before we
// can     // create the player (we need a username and uuid). Therefore, we
// need to do     // this on another task.
//     let worlds = self.worlds.clone();
//     tokio::spawn(async move {
//       worlds.new_player(req.into_inner(), tx).await;
//     });
//
//     Ok(Response::new(ReceiverStream::new(rx)))
//   }
//   async fn status(&self, req: Request<StatusRequest>) ->
// Result<Response<StatusResponse>, Status> {     dbg!(req);
//     Ok(Response::new(StatusResponse {
//       id:          vec![],
//       num_players: 5,
//       server_type: "bamboo".into(),
//     }))
//   }
//   async fn reserve_slots(
//     &self,
//     req: Request<ReserveSlotsRequest>,
//   ) -> Result<Response<ReserveSlotsResponse>, Status> {
//     dbg!(req);
//     Ok(Response::new(ReserveSlotsResponse::default()))
//   }
// }

fn main() {
  let args = Args::parse();
  let config = Config::new("server.toml", "server-default.toml", include_str!("default.toml"));

  let level = config.get("log-level");
  bb_common::init_with_level("server", level);

  if !args.no_docs {
    bb_server::generate_panda_docs();
    if args.only_docs {
      return;
    }
  }

  let addr = match config.get::<_, String>("address").parse() {
    Ok(v) => v,
    Err(e) => {
      error!("invalid address: {e}");
      return;
    }
  };

  let wm = Arc::new(WorldManager::new_with_config(config));
  wm.add_world();
  wm.load_plugins();

  if let Some(mut rcon) = RCon::new(wm.clone()) {
    thread::spawn(move || rcon.run());
  }

  let w = wm.clone();
  thread::spawn(|| w.run());

  let mut conn = ConnectionManager::new(wm);

  info!("listening on {}", addr);
  match conn.run(addr) {
    Ok(_) => {}
    Err(e) => error!("error in connection: {e}"),
  };
}
