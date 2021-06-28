use serde_derive::Deserialize;
use std::{collections::HashMap, fs::File};

#[derive(Deserialize)]
pub struct Settings {
  accounts: HashMap<String, Account>,
  selected: Selected,
}

#[derive(Deserialize)]
pub struct Account {
  access_token: String,
  profiles:     HashMap<String, AccountProfile>,
  username:     String,
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

impl Settings {
  /// Creates a new Settings struct, by loading all settings from disk.
  pub fn new() -> Self {
    serde_json::from_reader(File::open("~/.minecraft/launcher_profiles.json").unwrap()).unwrap()
  }

  /// Returns the current selected account's access token. Used to authenticate.
  pub fn get_token(&self) -> &str {
    &self.accounts[&self.selected.account].access_token
  }
}
