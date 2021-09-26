use std::error::Error;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  tokio::task::spawn_blocking(|| match sc_proxy::run() {
    Ok(_) => (),
    Err(e) => error!("error: {}", e),
  })
  .await?;

  Ok(())
}
