use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use common::proto::{
  minecraft_server::{Minecraft, MinecraftServer},
  Packet, ReserveSlotsRequest, ReserveSlotsResponse, StatusRequest, StatusResponse,
};

#[derive(Default)]
pub struct ServerImpl {}

#[tonic::async_trait]
impl Minecraft for ServerImpl {
  type ConnectionStream = ReceiverStream<Result<Packet, Status>>;
  async fn connection(
    &self,
    req: Request<Streaming<Packet>>,
  ) -> Result<Response<Self::ConnectionStream>, Status> {
    let (tx, rx) = mpsc::channel(4);

    dbg!(req);

    tokio::spawn(async move {
      let mut p = Packet::default();
      p.id = 10;
      println!("  => send {:?}", p);
      tx.send(Ok(p)).await.unwrap();

      println!(" /// done sending");
    });

    Ok(Response::new(ReceiverStream::new(rx)))
  }
  async fn status(&self, req: Request<StatusRequest>) -> Result<Response<StatusResponse>, Status> {
    dbg!(req);
    Ok(Response::new(StatusResponse {
      id: vec![],
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
  let addr = "0.0.0.0:8483".parse().unwrap();

  let svc = MinecraftServer::new(ServerImpl::default());
  let descriptor = tonic_reflection::server::Builder::configure()
    .register_encoded_file_descriptor_set(common::proto::FILE_DESCRIPTOR_SET)
    .build()?;

  Server::builder().add_service(svc).add_service(descriptor).serve(addr).await?;
  Ok(())
}
