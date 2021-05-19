#[macro_use]
extern crate log;

pub mod block;
pub mod item;
pub mod net;
pub mod player;
pub mod world;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use common::proto::{
  minecraft_server::{Minecraft, MinecraftServer},
  Packet, ReserveSlotsRequest, ReserveSlotsResponse, StatusRequest, StatusResponse,
};

use world::WorldManager;

#[derive(Clone)]
pub struct ServerImpl {
  worlds: WorldManager,
}

#[tonic::async_trait]
impl Minecraft for ServerImpl {
  type ConnectionStream = ReceiverStream<Result<Packet, Status>>;
  async fn connection(
    &self,
    req: Request<Streaming<Packet>>,
  ) -> Result<Response<Self::ConnectionStream>, Status> {
    let (tx, rx) = mpsc::channel(8);

    self.worlds.new_player(req.into_inner(), tx).await;

    Ok(Response::new(ReceiverStream::new(rx)))
  }
  async fn status(&self, req: Request<StatusRequest>) -> Result<Response<StatusResponse>, Status> {
    dbg!(req);
    Ok(Response::new(StatusResponse {
      id:          vec![],
      num_players: 5,
      server_type: "sugarcane-rs".into(),
    }))
  }
  async fn reserve_slots(
    &self,
    req: Request<ReserveSlotsRequest>,
  ) -> Result<Response<ReserveSlotsResponse>, Status> {
    dbg!(req);
    Ok(Response::new(ReserveSlotsResponse::default()))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  common::init();

  let addr = "0.0.0.0:8483".parse().unwrap();

  let svc = MinecraftServer::new(ServerImpl { worlds: WorldManager::new() });

  // This is the code needed for reflection. It is disabled for now, as
  // tonic-reflection does not allow you to disable rustfmt. For docker builds,
  // rustfmt is not installed.
  //
  // let desc = tonic_reflection::server::Builder::configure()
  //   .register_encoded_file_descriptor_set(common::proto::FILE_DESCRIPTOR_SET)
  //   .build()?;
  //
  // Server::builder()
  //   .add_service(svc)
  //   .add_service(desc)
  //   .serve(addr).await?;

  info!("listening on {}", addr);
  Server::builder().add_service(svc).serve(addr).await?;
  Ok(())
}
