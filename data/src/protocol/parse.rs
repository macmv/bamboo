use super::{json, BitField, CountType, FloatType, IntType, Packet, PacketField, Version};
use std::{collections::HashMap, error::Error, fs, path::Path};

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
    let types = generate_types(json.types);
    versions.insert(
      ver_str,
      Version {
        to_client: generate_list(json.play.to_client.types, &types),
        to_server: generate_list(json.play.to_server.types, &types),
      },
    );
  }

  Ok(versions)
}

fn generate_types(json: json::TypeMap) -> HashMap<String, PacketField> {
  let mut types = HashMap::new();

  for (k, v) in json {
    let mut ty = parse_type(v, &types);
    types.insert(k, ty);
  }

  types
}

fn generate_list(json: json::TypeMap, types: &HashMap<String, PacketField>) -> (Vec<Packet>) {
  let mut packets = vec![];

  for (k, v) in json {
    let mut fields = parse_type(v, &types).as_container().unwrap();
    packets.push(Packet { name: k, fields });
    // dbg!(k, v);
  }

  panic!();

  packets
}

fn parse_int(v: &str) -> Option<IntType> {
  match v {
    "i8" => Some(IntType::I8),
    "u8" => Some(IntType::U8),
    "i16" => Some(IntType::I16),
    "i32" => Some(IntType::I32),
    "i64" => Some(IntType::I64),
    "varint" => Some(IntType::VarInt),
    _ => None,
  }
}

fn parse_float(v: &str) -> Option<FloatType> {
  match v {
    "f32" => Some(FloatType::F32),
    "f64" => Some(FloatType::F64),
    _ => None,
  }
}

fn parse_count(v: json::CountType) -> CountType {
  match v {
    json::CountType::Typed(v) => CountType::Typed(parse_int(&v).unwrap()),
    json::CountType::Fixed(v) => CountType::Fixed(v),
    json::CountType::Named(v) => CountType::Named(v),
  }
}

fn parse_type(v: json::Type, types: &HashMap<String, PacketField>) -> PacketField {
  match *v.value {
    json::TypeValue::Direct(v) => match v.as_ref() {
      "bool" => PacketField::Bool,
      "UUID" => PacketField::UUID,
      "string" => PacketField::String,
      "position" => PacketField::Position,

      "nbt" => PacketField::NBT,
      "optionalNbt" => PacketField::OptionalNBT,
      "slot" => PacketField::Slot,
      "restBuffer" => PacketField::RestBuffer,
      "entityMetadata" => PacketField::RestBuffer,

      v => match parse_int(&v) {
        Some(v) => PacketField::Int(v),
        None => match parse_float(&v) {
          Some(v) => PacketField::Float(v),
          None => match types.get(v) {
            Some(v) => v.clone(),
            None => panic!("unknown field type: {}", v),
          },
        },
      },
    },
    json::TypeValue::Buffer(v) => PacketField::Buffer(parse_count(v.count_type)),
    json::TypeValue::Array(v) => {
      PacketField::Array { count: parse_count(v.count), value: Box::new(parse_type(v.ty, types)) }
    }
    json::TypeValue::Switch(v) => {
      PacketField::Switch { compare_to: v.compare_to, fields: HashMap::new() }
    }
    json::TypeValue::Option(v) => PacketField::Option(Box::new(parse_type(v, types))),
    json::TypeValue::Container(v) => {
      let mut fields = HashMap::new();
      for c in v {
        fields.insert(c.name.clone().unwrap(), parse_type(c.ty, types));
      }
      PacketField::Container(fields)
    }
    json::TypeValue::BitField(v) => {
      let mut fields = vec![];
      for f in v {
        fields.push(BitField { name: f.name, size: f.size, signed: f.signed });
      }
      PacketField::BitField(fields)
    }
    json::TypeValue::Mapper(v) => {
      let mut mappings = HashMap::new();
      for (k, v) in v.mappings {
        mappings.insert(
          k,
          u32::from_str_radix(&v, 16)
            .unwrap_or_else(|v| panic!("could not parse packet id: {}", v)),
        );
      }
      PacketField::Mappings(mappings)
    }
    v => panic!("invalid type. got: {:?}", v),
  }
}
