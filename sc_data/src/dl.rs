use serde::Deserialize;

pub fn get<T: Deserialize>(name: &str, ver: Version) -> T {
  let url = format!("https://macmv.gitlab.io/sugarcane-data/{}-{}.json", name, ver);
  let data = ureq::get(url).call().unwrap();
}
