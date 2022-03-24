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
use sugarlang::Sugarlang;

pub fn generate_sl_docs() {
  info!("generating sugarlang docs...",);

  let plugin = PandaPlugin::new(0, "".into(), Arc::new(WorldManager::new()));
  let mut sl = Sugarlang::new();
  plugin.add_builtins(&mut sl);
  plugin.generate_docs(&sl);

  info!(
    "generated docs at {}",
    std::env::current_dir().unwrap().join("target/sl_docs/sugarcane/index.html").display()
  );
}

// use std::sync::Arc;
// use tokio::sync::mpsc;
// use tokio_stream::wrappers::ReceiverStream;
// use tonic::{Request, Response, Status, Streaming};
//
// use sc_common::proto::{
//   minecraft_server::Minecraft, Packet, ReserveSlotsRequest,
// ReserveSlotsResponse, StatusRequest,   StatusResponse,
// };
//
// use world::WorldManager;

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
//       server_type: "sugarcane-rs".into(),
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
