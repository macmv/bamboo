use super::{Packet, Version};
use serde_derive::Deserialize;
use std::{collections::HashMap, error::Error, fs, path::Path};

type JsonTypeMap = HashMap<String, JsonType>;
type JsonType = Vec<JsonTypeField>;

#[derive(Debug, Deserialize)]
struct JsonArray {
  #[serde(alias = "countType")]
  count_type: String,
  #[serde(alias = "type")]
  ty:         JsonType,
}

#[derive(Debug, Deserialize)]
struct JsonObject {
  name: String,
}

#[derive(Debug, Deserialize)]
struct JsonAnonObject {
  anon: bool,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonTypeField {
  // This is "container" or "array". We ignore it, as the type is infered because enums
  Kind(String),
  // A container. This is a list of objects
  Container(JsonType),
  // This is a list of fields, with a count type
  Array(JsonArray),
  // This is an actual object
  Object(JsonObject),
  // This is an actual object
  AnonObject(JsonAnonObject),
}

#[derive(Debug, Deserialize)]
struct JsonPacketMap {
  // This is a map of length two arrays
  // The first element is a JsonPacketEntry::Kind, and the second is a JsonPacketEntry::Value.
  types: JsonTypeMap,
}

#[derive(Debug, Deserialize)]
struct JsonClientServerMap {
  #[serde(alias = "toClient")]
  to_client: JsonPacketMap,
  #[serde(alias = "toServer")]
  to_server: JsonPacketMap,
}

#[derive(Debug, Deserialize)]
struct JsonProtocolVersion {
  // types: HashMap<String, JsonTypeDef>,
  handshaking: JsonClientServerMap,
  status:      JsonClientServerMap,
  login:       JsonClientServerMap,
  play:        JsonClientServerMap,
}

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

    let fname = p.join("protocol.json");
    let json: JsonProtocolVersion = match serde_json::from_str(&fs::read_to_string(&fname)?) {
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
    panic!();
  }

  Ok(versions)
}

fn generate_list(json: &JsonTypeMap) -> Vec<Packet> {
  let packets = vec![];

  for (k, v) in json {
    dbg!(k, v);
  }

  packets
}
