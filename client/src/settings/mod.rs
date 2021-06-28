use common::util::UUID;
use directories::BaseDirs;
use serde_derive::Deserialize;
use std::{collections::HashMap, fs::File, path::PathBuf, str::FromStr};

#[derive(Deserialize)]
pub struct Settings {
  accounts: HashMap<String, Account>,
  selected: Selected,
}

#[derive(Deserialize)]
pub struct Account {
  access_token: String,
  // Keyed by account UUID
  profiles:     HashMap<String, AccountProfile>,
  // Email address (not username)
  username:     String,
}

#[derive(Deserialize)]
pub struct AccountProfile {
  display_name: String,
}

#[derive(Deserialize)]
pub struct Selected {
  // The selcted account
  account: String,
  // The selected profile (an installation)
  profile: String,
}

/// An easier to use version of the account data.
pub struct AccountInfo {
  uuid:         UUID,
  username:     String,
  access_token: String,
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
    info!("got dir {:?}", dir);
    serde_json::from_reader(File::open("~/.minecraft/launcher_profiles.json").unwrap()).unwrap()
  }

  /// Returns the selected account info.
  pub fn get_info(&self) -> AccountInfo {
    let account = &self.accounts[&self.selected.account];
    let (uuid, profile) = account.profiles.iter().next().unwrap();
    AccountInfo {
      uuid:         UUID::from_str(uuid).unwrap(),
      username:     profile.display_name.clone(),
      access_token: account.access_token.clone(),
    }
  }
}
