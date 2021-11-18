use crate::Version;
use serde::de::DeserializeOwned;
use std::{fs, fs::File, io};

pub fn get<T: DeserializeOwned>(name: &str, ver: Version) -> T {
  let dir = crate::out_dir().join("data");
  if !dir.exists() {
    fs::create_dir_all(&dir)
      .unwrap_or_else(|e| panic!("could not create dir {}: {}", dir.display(), e));
  }
  let p = dir.join(format!("{}-{}.json", name, ver));
  if p.exists() {
    println!("file {} already exists, skipping download", p.display());
    let f = File::open(&p).unwrap_or_else(|e| panic!("cannot open file at {}: {}", p.display(), e));
    if let Ok(res) = serde_json::from_reader(f) {
      return res;
    } else {
      println!("file {} has invalid json, redownloading", p.display());
    }
  }

  let url = format!("https://macmv.gitlab.io/sugarcane-data/{}-{}.json", name, ver);
  let data = ureq::get(&url).call().unwrap();

  println!("download file {} from {}", p.display(), url);
  let mut f =
    File::create(&p).unwrap_or_else(|e| panic!("cannot create file at {}: {}", p.display(), e));
  io::copy(&mut data.into_reader(), &mut f)
    .unwrap_or_else(|e| panic!("could not write file {}: {}", p.display(), e));

  let f = File::open(&p).unwrap_or_else(|e| panic!("cannot open file at {}: {}", p.display(), e));
  serde_json::from_reader(f).unwrap()
}
