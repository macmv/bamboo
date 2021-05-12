use super::{Packet, Version};
use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
use serde_derive::Deserialize;
use std::{collections::HashMap, error::Error, fmt, fs, path::Path};

type JsonTypeMap = HashMap<String, JsonType>;

#[derive(Debug)]
struct JsonType {
  kind:  String,
  value: Box<JsonTypeValue>,
}

impl<'de> Deserialize<'de> for JsonType {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct Arr;

    macro_rules! match_values {
      ($self:expr, $k:expr, $s:expr, [$($name:expr, $kind:ident),*]) => {
        match $k {
          $(
            $name => Ok(JsonTypeValue::$kind($s.next_element()?.ok_or_else(|| de::Error::invalid_length(1, $self))?)),
          )*
          v => Err(de::Error::unknown_variant(v, &[$($name),*]))
        }
      };
    }

    impl<'de> Visitor<'de> for Arr {
      type Value = JsonType;

      fn expecting(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "a string")
      }

      fn visit_str<E>(self, mut s: &str) -> Result<Self::Value, E>
      where
        E: de::Error,
      {
        Ok(JsonType { kind: "direct".into(), value: Box::new(JsonTypeValue::Direct(s.into())) })
      }

      fn visit_seq<S>(self, mut s: S) -> Result<Self::Value, S::Error>
      where
        S: SeqAccess<'de>,
      {
        let kind = s.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let value = match_values!(
          &self,
          kind,
          s,
          [
            "container",
            Container,
            "array",
            Array,
            "mapper",
            Mapper,
            "switch",
            Switch,
            "buffer",
            Buffer,
            "option",
            Option,
            "bitfield",
            Bitfield
          ]
        )?;
        Ok(JsonType { kind: kind.into(), value: Box::new(value) })
      }
    }

    deserializer.deserialize_any(Arr)
  }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonTypeValue {
  // Just a string type (will be something like "varint")
  Direct(String),
  // A container. This is a list of objects
  Container(Vec<JsonContainer>),
  // This is a list of fields, with a count type
  Array(JsonArray),
  // This is a list of mappings. TODO: Learn more
  Mapper(JsonMapper),
  // This is a list of conditionals and values. TODO: Learn more
  Switch(JsonSwitch),
  // This is a byte array, with a length type
  Buffer(JsonBuffer),
  // This is a value that may or may not exist
  Option(Option<JsonType>),
  // This is a value that may or may not exist
  Bitfield(Vec<JsonBitfield>),
}

#[derive(Debug, Deserialize)]
struct JsonBitfield {
  name:   String,
  size:   u32,
  signed: bool,
}

#[derive(Debug, Deserialize)]
struct JsonContainer {
  name: String,
  #[serde(alias = "type")]
  ty:   JsonType,
}

// If count_type is none, then it is a fixed length array of length count. If
// count is none, then this is an array prefixed with a value of the given type.
// If both are none, this is invalid.
#[derive(Debug, Deserialize)]
struct JsonArray {
  #[serde(alias = "countType")]
  count_type: Option<JsonType>,
  count:      Option<u32>,
  #[serde(alias = "type")]
  ty:         JsonType,
}

#[derive(Debug, Deserialize)]
struct JsonObject {
  name: String,
  #[serde(alias = "type")]
  ty:   JsonType,
}

#[derive(Debug, Deserialize)]
struct JsonAnonObject {
  anon: bool,
  #[serde(alias = "type")]
  ty:   JsonType,
}

#[derive(Debug, Deserialize)]
struct JsonMapper {
  mappings: HashMap<String, String>,
  #[serde(alias = "type")]
  ty:       JsonType,
}

#[derive(Debug, Deserialize)]
struct JsonSwitch {
  // Another field name to be compared with
  #[serde(alias = "compareTo")]
  compare_to: String,
  fields:     HashMap<String, JsonType>,
}

#[derive(Debug, Deserialize)]
struct JsonBuffer {
  // The type that the length is in
  #[serde(alias = "countType")]
  count_type: String,
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
