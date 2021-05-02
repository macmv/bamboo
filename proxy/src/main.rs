use common::proto::{
  minecraft_client::MinecraftClient, Packet, ReserveSlotsRequest, ReserveSlotsResponse,
  StatusRequest, StatusResponse,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut client = MinecraftClient::connect("http://0.0.0.0:8483").await?;

  let req = tonic::Request::new(StatusRequest {});

  println!("Sending request to gRPC Server...");
  let res = client.status(req).await?;

  dbg!(res);

  Ok(())
}
