use crate::Version;
use serde::{de::DeserializeOwned, Deserialize};
use std::{collections::HashMap, fs, fs::File, io, io::Read, path::Path};

#[derive(Deserialize)]
#[cfg_attr(test, allow(dead_code))]
struct Config {
  sources: SourcesMode,

  url:  Option<String>,
  path: Option<String>,
}

#[derive(Deserialize)]
#[cfg_attr(test, allow(dead_code))]
enum SourcesMode {
  #[serde(rename = "local")]
  Local,
  #[serde(rename = "remote")]
  Remote,
}

#[cfg_attr(test, allow(dead_code))]
pub struct Downloader {
  files: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl Downloader {
  pub fn new() -> Self {
    // Our current directory is within the project we are building for (for example,
    // we would be inside the `bb_server` directory if compiling for `bb_server`).
    // The config is outside that, so we prefix with `../`.
    println!("cargo:rerun-if-changed=../data-config.toml");
    let config: Config = if Path::new("../data-config.toml").exists() {
      toml::from_str(&fs::read_to_string("../data-config.toml").unwrap()).unwrap()
    } else {
      toml::from_str(&fs::read_to_string("../data-config-example.toml").unwrap()).unwrap()
    };
    let mut buf = vec![];
    match config.sources {
      SourcesMode::Remote => {
        let url = config.url.as_ref().expect("url must be present for remote bamboo-data");
        let url = format!("{url}/all-releases.json.gz");

        let data = ureq::get(&url).call().unwrap();

        io::copy(&mut data.into_reader(), &mut buf)
          .unwrap_or_else(|e| panic!("could not download bamboo-data json: {e}"));
      }
      SourcesMode::Local => {
        let path = config.path.as_ref().expect("path must be present for local bamboo-data");
        let path = Path::new("../").join(path).join("all-releases.json.gz");

        let mut data = File::open(&path).unwrap();

        io::copy(&mut data, &mut buf)
          .unwrap_or_else(|e| panic!("could not copy bamboo-data json: {e}"));
      }
    };

    let mut reader = flate2::read::GzDecoder::new(&*buf);
    let mut uncompressed = vec![];
    reader.read_to_end(&mut uncompressed).expect("could not decompress bamboo-data json");

    Downloader { files: serde_json::from_reader(&*uncompressed).unwrap() }
  }

  #[track_caller]
  pub fn get<T: DeserializeOwned>(&self, name: &str, ver: Version) -> T {
    /*
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
    */

    serde_json::from_value(self.files[&ver.to_string()][name].clone()).unwrap()
  }
}
