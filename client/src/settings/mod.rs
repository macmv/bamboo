use common::util::UUID;
use directories::BaseDirs;
use serde_derive::Deserialize;
use std::{collections::HashMap, fs::File, str::FromStr};

#[derive(Deserialize)]
pub struct Settings {
  #[serde(rename = "authenticationDatabase")]
  accounts: HashMap<String, Account>,
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
  /// Creates a new Settings struct, by loading all settings from disk.
  pub fn new() -> Self {
    let dirs = BaseDirs::new().unwrap();
    let mut dir = dirs.config_dir().to_path_buf();
    if dir.ends_with(".config") {
      // Mojang has never used linux before (they use ~/.minecraft instead of
      // ~/.config/.minecraft)
      dir = dir.parent().unwrap().join(".minecraft");
    } else if dir.ends_with("Application Support") {
      // On macos there is no '.' at the start
      dir = dir.join("minecraft");
    } else {
      dir = dir.join(".minecraft");
    }
    info!("using data directory {:?}", dir);
    serde_json::from_reader(File::open(dir.join("launcher_profiles.json")).unwrap()).unwrap()
  }

  /// Returns the selected account info.
  pub fn get_info(&self) -> AccountInfo {
    let account = &self.accounts[&self.selected.account];
    let profile = &account.profiles[&self.selected.profile];
    AccountInfo {
      uuid:         UUID::from_str(&self.selected.profile).unwrap(),
      username:     profile.display_name.clone(),
      access_token: account.access_token.clone(),
    }
  }
}
