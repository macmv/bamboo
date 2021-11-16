use crate::Version;
use serde::de::DeserializeOwned;

pub fn get<T: DeserializeOwned>(name: &str, ver: Version) -> T {
  let url = format!("https://macmv.gitlab.io/sugarcane-data/{}-{}.json", name, ver);
  let data = ureq::get(&url).call().unwrap();
  serde_json::from_reader(data.into_reader()).unwrap()
}
