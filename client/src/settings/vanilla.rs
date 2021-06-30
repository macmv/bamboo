use common::util::UUID;
use directories::BaseDirs;
use serde_derive::Deserialize;
use std::{collections::HashMap, fs::File, str::FromStr};

use super::LoginInfo;

#[derive(Deserialize)]
struct Profile {
  #[serde(rename = "authenticationDatabase")]
  accounts: HashMap<String, Account>,
  #[serde(rename = "selectedUser")]
  selected: Selected,
}

#[derive(Deserialize)]
struct Account {
  #[serde(rename = "accessToken")]
  access_token: String,
  // Keyed by account UUID
  profiles:     HashMap<String, AccountProfile>,
  // Email address (not username)
  username:     String,
}

#[derive(Deserialize)]
struct AccountProfile {
  #[serde(rename = "displayName")]
  display_name: String,
}

#[derive(Deserialize)]
struct Selected {
  // The selcted account
  account: String,
  // The selected profile (account uuid)
  profile: String,
}

pub fn get_login_info() -> Option<LoginInfo> {
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
  if dir.exists() {
    let path = dir.join("launcher_profiles.json");
    let launcher: Profile = match serde_json::from_reader(match File::open(&path) {
      Ok(f) => f,
      Err(e) => {
        warn!("error while reading vanilla login information: {}", e);
        return None;
      }
    }) {
      Ok(v) => v,
      Err(e) => {
        warn!(
          "found vanilla login information at {}, but the json was invalid: {}",
          path.to_str().unwrap(),
          e
        );
        return None;
      }
    };
    let account = launcher.accounts.get(&launcher.selected.account).unwrap();
    let selected_profile = &account.profiles[&launcher.selected.profile];
    Some(LoginInfo {
      uuid:         UUID::from_str(&launcher.selected.profile).unwrap(),
      username:     selected_profile.display_name.clone(),
      access_token: account.access_token.clone(),
      client_token: launcher.selected.account,
    })
  } else {
    None
  }
}
