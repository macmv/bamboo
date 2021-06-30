use common::util::UUID;
use directories::BaseDirs;
use serde_derive::Deserialize;
use std::{collections::HashMap, fs::File, process, str::FromStr};

mod auth;

#[derive(Deserialize)]
pub struct Settings {
  #[serde(rename = "authenticationDatabase")]
  accounts: Option<HashMap<String, Account>>,
  #[serde(rename = "selectedUser")]
  selected: Selected,
}

#[derive(Deserialize)]
pub struct Account {
  #[serde(rename = "accessToken")]
  access_token: String,
  // Keyed by account UUID
  profiles:     HashMap<String, AccountProfile>,
  // Email address (not username)
  username:     String,
}

#[derive(Deserialize)]
pub struct AccountProfile {
  #[serde(rename = "displayName")]
  display_name: String,
}

#[derive(Deserialize)]
pub struct Selected {
  // The selcted account
  account: String,
  // The selected profile (account uuid)
  profile: String,
}

/// An easier to use version of the account data.
#[derive(Debug)]
pub struct AccountInfo {
  uuid:         UUID,
  username:     String,
  access_token: String,
}

impl AccountInfo {
  pub fn uuid(&self) -> UUID {
    self.uuid
  }
  pub fn username(&self) -> &str {
    &self.username
  }
  pub fn access_token(&self) -> &str {
    &self.access_token
  }
}

impl Settings {
  /// Creates a new Settings struct, by loading all settings from disk. The
  /// second field is if the client needs to be logged in. If this returns true,
  /// then the UI should bring up a login screen, and disallow joining any
  /// servers.
  pub fn new() -> Self {
    let dirs = BaseDirs::new().unwrap();
    let mut dir = dirs.config_dir().to_path_buf();
    if dir.ends_with(".config") {
      // Mojang has never used linux before (they use ~/.minecraft instead of
      // ~/.config/minecraft)
      dir = dir.parent().unwrap().join(".minecraft");
    } else if dir.ends_with("Application Support") {
      // On macos there is no '.' at the start
      dir = dir.join("minecraft");
    } else {
      dir = dir.join(".minecraft");
    }
    if !dir.exists() {
      error!(
        "{} does not exist! please create it with the vanilla launcher",
        dir.to_str().unwrap()
      );
      process::exit(1);
    }
    info!("using data directory {:?}", dir);
    let path = dir.join("launcher_profiles.json");
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
  }

  /// Returns true if there is a valid auth token in the launcher profiles. If
  /// this returns false, then [`get_info`](Self::get_info) will panic when you
  /// call this. If this contains an out-of-date token, then this function will
  /// update that token. It will not write to disk, but it will change the
  /// stored access token.
  pub async fn login(&mut self) -> bool {
    let account = match &mut self.accounts {
      Some(v) => v.get_mut(&self.selected.account).unwrap(),
      None => return false,
    };

    let (new_token, valid) =
      match auth::refresh_token(&account.access_token, &self.selected.account).await {
        Ok(v) => v,
        Err(e) => {
          error!("could not refresh auth token: {}", e);
          return false;
        }
      };
    if let Some(new_token) = new_token {
      account.access_token = new_token;
    }

    valid
  }

  /// Returns the selected account info.
  pub fn get_info(&self) -> AccountInfo {
    let account = match &self.accounts {
      Some(v) => v.get(&self.selected.account).unwrap(),
      None => panic!("no valid account stored!"),
    };
    let profile = &account.profiles[&self.selected.profile];
    AccountInfo {
      uuid:         UUID::from_str(&self.selected.profile).unwrap(),
      username:     profile.display_name.clone(),
      access_token: account.access_token.clone(),
    }
  }
}
