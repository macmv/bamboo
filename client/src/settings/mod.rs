use common::util::UUID;
use serde_derive::Deserialize;
use std::{collections::HashMap, fs::File, str::FromStr};

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
  username:     String, // This is your email
}

#[derive(Deserialize)]
pub struct AccountProfile {
  display_name: String,
}

#[derive(Deserialize)]
pub struct Selected {
  account: String,
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
