use common::util::UUID;
use directories::BaseDirs;
use serde_derive::Deserialize;
use std::{fs::File, process};

mod auth;
mod vanilla;

#[derive(Deserialize)]
pub struct Settings {
  login: Option<LoginInfo>,
}

/// Stores all of the info needed to login to a server.
#[derive(Debug, Deserialize)]
pub struct LoginInfo {
  uuid:         UUID,
  username:     String,
  access_token: String,
  client_token: String,
}

impl LoginInfo {
  pub fn uuid(&self) -> UUID {
    self.uuid
  }
  pub fn username(&self) -> &str {
    &self.username
  }
  pub fn access_token(&self) -> &str {
    &self.access_token
  }
  pub fn client_token(&self) -> &str {
    &self.client_token
  }
}

impl Settings {
  /// Creates a new Settings struct, by loading all settings from disk. The
  /// second field is if the client needs to be logged in. If this returns true,
  /// then the UI should bring up a login screen, and disallow joining any
  /// servers.
  pub fn new() -> Self {
    let dirs = BaseDirs::new().unwrap();
    let mut dir = dirs.config_dir().join("sugarcane").to_path_buf();
    info!("using data directory {:?}", dir);
    let path = dir.join("settings.json");
    if path.exists() {
      match serde_json::from_reader(match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
          error!("error while reading from {}: {}", &path.to_str().unwrap(), e);
          process::exit(1);
        }
      }) {
        Ok(v) => v,
        Err(e) => {
          error!("error while parsing json in {}: {}", path.to_str().unwrap(), e);
          process::exit(1);
        }
      }
    } else {
      Settings { login: vanilla::get_login_info() }
    }
  }

  /// Returns true if there is a valid auth token in the launcher profiles. If
  /// this returns false, then [`get_login`](Self::get_login) will panic when
  /// callled. If this contains an out-of-date token, then this function will
  /// update that token. It will not write to disk, but it will change
  /// the stored access token.
  pub async fn refresh_token(&mut self) -> bool {
    match self.login {
      Some(ref mut info) => info.refresh_token().await,
      None => false,
    }
  }

  /// This creates a new access token. It will panic if there is already a valid
  /// access token. Returns true if the login was successful, false if
  /// otherwise.
  pub async fn login(&mut self, username: &str, password: &str) -> bool {
    if self.login.is_some() {
      panic!("called login with valid login info");
    }
    self.login = LoginInfo::new(username, password).await;
    self.login.is_some()
  }

  /// Returns the selected account info.
  pub fn get_login(&self) -> &LoginInfo {
    self.login.as_ref().unwrap()
  }
}

impl LoginInfo {
  async fn new(username: &str, password: &str) -> Option<Self> {
    match auth::login(username, password).await {
      Ok(v) => v,
      Err(e) => {
        error!("failed to login: {}", e);
        None
      }
    }
  }
  async fn refresh_token(&mut self) -> bool {
    match auth::validate_token(&self.access_token, &self.client_token).await {
      Ok(true) => return true,
      Err(e) => {
        error!("could not validate auth token: {}", e);
        return false;
      }
      _ => {}
    }
    let (new_token, valid) = match auth::refresh_token(&self.access_token, &self.client_token).await
    {
      Ok(v) => v,
      Err(e) => {
        error!("could not refresh auth token: {}", e);
        return false;
      }
    };
    if let Some(new_token) = new_token {
      self.access_token = new_token;
    }

    valid
  }
}
