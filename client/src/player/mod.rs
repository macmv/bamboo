use crate::Settings;
use common::util::UUID;

/// The player that the client is using. This include information about
/// rendering, the camera position, and anything else that is client specific.
pub struct MainPlayer {
  info: OtherPlayer,
}

/// This is a struct used for any player. This includes logic for parsing
/// packets coming from the server, and how to render a player model.
pub struct OtherPlayer {
  name: String,
  uuid: UUID,
}

impl MainPlayer {
  pub fn new(settings: &Settings) -> Self {
    let info = settings.get_info();
    MainPlayer { info: OtherPlayer::new(info.username(), info.uuid()) }
  }
}

impl OtherPlayer {
  pub fn new(name: &str, uuid: UUID) -> Self {
    OtherPlayer { name: name.into(), uuid }
  }
}
