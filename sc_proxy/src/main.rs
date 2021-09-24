use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  sc_proxy::run().await
}
