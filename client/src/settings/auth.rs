use reqwest::{Client, StatusCode};
use serde_derive::{Deserialize, Serialize};
use std::io::{self, ErrorKind};

#[derive(Deserialize)]
struct Profile {
  // UUID of the profile
  id:   String,
  // Email address
  name: String,
}

#[derive(Serialize)]
struct Refresh {
  access_token: String,
  client_token: String,
  request_user: bool,
}

#[derive(Deserialize)]
struct RefreshResp {
  // The new access token
  access_token:     String,
  // Identical to the sent client token
  client_token:     String,
  selected_profile: Profile,
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
