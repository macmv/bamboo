use common::util::UUID;
use reqwest::{Client, StatusCode};
use serde_derive::{Deserialize, Serialize};
use std::{
  io::{self, ErrorKind},
  str::FromStr,
};

use super::LoginInfo;

#[derive(Debug, Serialize)]
struct Agent {
  // Should always be 'Minecraft'
  name:    String,
  // Should always be 1
  version: u32,
}

#[derive(Debug, Serialize)]
struct Authenticate {
  agent:        Agent,
  username:     String,
  password:     String,
  client_token: String,
}

#[derive(Debug, Deserialize)]
struct Profile {
  // UUID of the profile
  id:   String,
  // Email address
  name: String,
}

#[derive(Debug, Deserialize)]
struct Prop {
  // Something like their country, language, etc.
  name:  String,
  // Something like "en-us" or "US"
  value: String,
}

#[derive(Debug, Deserialize)]
struct User {
  // Email
  username: String,
  props:    Vec<Prop>,
  // UUID
  id:       String,
}

#[derive(Debug, Deserialize)]
struct AuthenticateResp {
  user:         User,
  // Identical to the sent client token
  client_token: String,
  access_token: String,
  available:    Vec<Profile>,
  selected:     Profile,
}

#[derive(Debug, Serialize)]
struct Refresh {
  access_token: String,
  client_token: String,
  request_user: bool,
}

#[derive(Debug, Deserialize)]
struct RefreshResp {
  // The new access token
  access_token:     String,
  // Identical to the sent client token
  client_token:     String,
  selected_profile: Profile,
}

/// Creates a new access and client token for the given user.
///
/// # Return values
///
/// - `Err(_)` The auth server gave an unexpected response.
/// - `Ok(None)` The username/password are invalid.
/// - `Ok(Some_)` The client was successfully logged in.
pub async fn login(username: &str, password: &str) -> Result<Option<LoginInfo>, io::Error> {
  let client = Client::new();
  match client
    .post("https://authserver.mojang.com/authenticate")
    .json(&Authenticate {
      agent:        Agent { name: "Mojang".into(), version: 1 },
      username:     username.into(),
      password:     password.into(),
      client_token: "".into(),
    })
    .send()
    .await
  {
    Ok(res) => {
      if res.status() != StatusCode::OK {
        return Err(io::Error::new(ErrorKind::Other, res.text().await.unwrap()));
      }
      let out: AuthenticateResp = match res.json().await {
        Ok(v) => v,
        Err(e) => {
          return Err(io::Error::new(ErrorKind::Other, e));
        }
      };
      dbg!(&out);
      Ok(Some(LoginInfo {
        access_token: out.access_token,
        client_token: out.client_token,
        username:     out.selected.name,
        uuid:         UUID::from_str(&out.selected.id).unwrap(),
      }))
    }
    Err(e) => return Err(io::Error::new(ErrorKind::Other, e)),
  }
}

/// Updates the given auth token. The returned string is a new auth token, as
/// the client token never changes.
///
/// # Return values
///
/// - `(None, false)` The auth token is invalid. The user must login with an
///   email and password.
/// - `(Some(_), true)` The auth token is out of date. The returned string is
///   the new token.
/// - `(None, true)` The auth token is up to date.
pub async fn refresh_token(
  access_token: &str,
  client_token: &str,
) -> Result<(Option<String>, bool), io::Error> {
  let client = Client::new();
  match client
    .post("https://authserver.mojang.com/refresh")
    .json(&Refresh {
      access_token: access_token.into(),
      client_token: client_token.into(),
      request_user: false,
    })
    .send()
    .await
  {
    Ok(res) => {
      if res.status() != StatusCode::NO_CONTENT {
        return Err(io::Error::new(ErrorKind::Other, res.text().await.unwrap()));
      }
      let out: RefreshResp = match res.json().await {
        Ok(v) => v,
        Err(e) => {
          return Err(io::Error::new(ErrorKind::Other, e));
        }
      };
      if out.access_token == access_token {
        Ok((None, true))
      } else {
        Ok((Some(out.access_token), true))
      }
    }
    Err(e) => return Err(io::Error::new(ErrorKind::Other, e)),
  }
}
