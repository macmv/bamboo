#[macro_use]
extern crate log;

fn main() {
  match sc_proxy::run() {
    Ok(_) => (),
    Err(e) => error!("error: {}", e),
  }
}
