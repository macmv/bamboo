use crate::Version;
use serde::{de::DeserializeOwned, Deserialize};
use std::{
  fs,
  fs::File,
  io,
  path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct Config {
  sources: SourcesMode,

  url:  Option<String>,
  path: Option<String>,
}

#[derive(Deserialize)]
enum SourcesMode {
  #[serde(rename = "local")]
  Local,
  #[serde(rename = "remote")]
  Remote,
}

pub struct Downloader {
  config: Config,
  out:    PathBuf,
}

impl Downloader {
  pub fn new(out: PathBuf) -> Self {
    // Our current directory is within the project we are building for (for example,
    // we would be inside the `bb_server` directory if compiling for `bb_server`).
    // The config is outside that, so we prefix with `../`.
    Downloader {
      config: if Path::new("../data-config.toml").exists() {
        toml::from_str(&fs::read_to_string("../data-config.toml").unwrap()).unwrap()
      } else {
        toml::from_str(&fs::read_to_string("../data-config-example.toml").unwrap()).unwrap()
      },
      out,
    }
  }

  #[cfg(test)]
  pub fn get<T: DeserializeOwned>(&self, name: &str, ver: Version) -> T {
    let url = format!("https://macmv.gitlab.io/bamboo-data/{}-{}.json", name, ver);
    let data = ureq::get(&url).call().unwrap();
    serde_json::from_reader(data.into_reader()).unwrap()
  }

  #[cfg(not(test))]
  pub fn get<T: DeserializeOwned>(&self, name: &str, ver: Version) -> T {
    let cache_dir = self.out.join("data");
    if !cache_dir.exists() {
      fs::create_dir_all(&cache_dir)
        .unwrap_or_else(|e| panic!("could not create cache dir {}: {}", cache_dir.display(), e));
    }
    let p = cache_dir.join(format!("{}-{}.json", name, ver));
    if p.exists() {
      println!("file {} already exists, skipping download", p.display());
      let f =
        File::open(&p).unwrap_or_else(|e| panic!("cannot open file at {}: {}", p.display(), e));
      if let Ok(res) = serde_json::from_reader(f) {
        return res;
      } else {
        println!("file {} has invalid json, redownloading", p.display());
      }
    }

    match self.config.sources {
      SourcesMode::Remote => {
        let url = self.config.url.as_ref().expect("url must be present for remote download");
        let url = format!("{url}/{name}-{ver}.json");

        let data = ureq::get(&url).call().unwrap();

        println!("download file {} from {}", p.display(), url);
        let mut f = File::create(&p)
          .unwrap_or_else(|e| panic!("cannot create file at {}: {}", p.display(), e));
        io::copy(&mut data.into_reader(), &mut f)
          .unwrap_or_else(|e| panic!("could not write file {}: {}", p.display(), e));
      }
      SourcesMode::Local => {
        let path = self.config.path.as_ref().expect("path must be present for local sources");
        let path = Path::new("../").join(path).join(format!("{name}-{ver}.json"));

        let mut data = File::open(&path).unwrap();

        println!("copying file {} from {}", p.display(), path.display());
        let mut f = File::create(&p)
          .unwrap_or_else(|e| panic!("cannot create file at {}: {}", p.display(), e));
        io::copy(&mut data, &mut f)
          .unwrap_or_else(|e| panic!("could not write file {}: {}", p.display(), e));
      }
    }

    let f = File::open(&p).unwrap_or_else(|e| panic!("cannot open file at {}: {}", p.display(), e));
    serde_json::from_reader(f).unwrap()
  }
}
