use super::{json, Packet, Version};
use std::{collections::HashMap, error::Error, fmt, fs, path::Path};

pub(super) fn load_all(path: &Path) -> Result<HashMap<String, Version>, Box<dyn Error>> {
  let mut versions = HashMap::new();

  dbg!(path);
  for p in fs::read_dir(path).unwrap() {
    let p = p.unwrap().path();
    let name = p.file_name().unwrap().to_str().unwrap();
    // If it contains any letters, then it is not a full release
    if name.chars().any(char::is_alphabetic) {
      continue;
    }
    // This generates a string which is one of the ProtocolVersion enums
    let sections: Vec<&str> = name.split('.').collect();
    let ver_str;
    if sections.len() == 2 {
      ver_str = format!("V{}_{}", sections[0], sections[1]);
    } else if sections.len() == 3 {
      ver_str = format!("V{}_{}_{}", sections[0], sections[1], sections[2]);
    } else {
      continue;
    }
    // 1.7 has incompatibilities that I couldn't be bothered to fix
    if ver_str == "V1_7" {
      continue;
    }

    let fname = p.join("protocol.json");
    let file = match fs::read_to_string(&fname) {
      Ok(v) => v,
      Err(_) => continue,
    };
    let json: json::ProtocolVersion = match serde_json::from_str(&file) {
      Ok(v) => v,
      Err(e) => panic!("while reading file {}, got json error {}", fname.display(), e),
    };

    println!("ver: {}", &ver_str);
    versions.insert(
      ver_str,
      Version {
        to_client: generate_list(&json.play.to_client.types),
        to_server: generate_list(&json.play.to_server.types),
      },
    );
  }

  Ok(versions)
}

fn generate_list(json: &json::TypeMap) -> Vec<Packet> {
  let packets = vec![];

  for (k, v) in json {
    // dbg!(k, v);
  }

  packets
}
