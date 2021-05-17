use super::{json, BitField, CountType, FloatType, IntType, Packet, PacketField, Version};
use convert_case::{Case, Casing};
use std::{collections::HashMap, error::Error, fs, io, io::ErrorKind, path::Path};

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
    let mut types = HashMap::new();
    generate_types(json.types, &mut types)?;
    versions.insert(
      ver_str,
      Version {
        to_client: match generate_packets(json.play.to_client.types, types.clone()) {
          Ok(v) => v.into_iter().map(|v| v.unwrap()).collect(),
          Err(e) => panic!("error while parsing clientbound packets for version {}: {}", &name, e),
        },
        to_server: match generate_packets(json.play.to_server.types, types.clone()) {
          Ok(v) => v.into_iter().map(|v| v.unwrap()).collect(),
          Err(e) => panic!("error while parsing serverbound packets for version {}: {}", &name, e),
        },
        types,
      },
    );
  }

  Ok(versions)
}

// Some types are defined in the to_client and to_server fields. In order to
// generate those types first, we must run two passes. The first pass will
// generate all types (parse_packets: false). And the second pass will generate
// all packets (parse_packets: true).
fn generate_types(json: json::TypeMap, types: &mut HashMap<String, PacketField>) -> io::Result<()> {
  for (k, v) in json {
    // String is not defined as a native type, so we get some issues with pstring
    // existing.
    if v.kind == "pstring" {
      continue;
    }
    types.insert(k, parse_type(v, &types));
  }
  Ok(())
}

fn generate_packets(
  json: json::TypeMap,
  mut types: HashMap<String, PacketField>,
) -> io::Result<Vec<Option<Packet>>> {
  generate_types(json, &mut types)?;
  let mut packets = vec![];

  match types.get("packet") {
    Some(PacketField::Container(values)) => {
      let names = match &values["name"] {
        PacketField::Mappings(v) => v,
        _ => {
          return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("expected a mappings type, got: {:?}", values["name"]),
          ))
        }
      };
      let params = match &values["params"] {
        PacketField::Switch { compare_to: _, fields } => fields,
        _ => {
          return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("expected a switch, got: {:?}", values["name"]),
          ))
        }
      };
      for (short_name, id) in names {
        // short_name is something like 'teams', while long_name is something like
        // 'packet_teams'.
        let long_name = params[short_name].clone().into_defined().unwrap().clone();
        // The packet id
        let id = *id as usize;
        if id >= packets.len() {
          for _ in packets.len()..id + 1 {
            packets.push(None);
          }
        }
        packets[id] = Some(Packet {
          fields: types[&long_name].clone().into_container().unwrap(),
          name:   short_name.into(),
        });
      }
    }
    _ => return Err(io::Error::new(ErrorKind::InvalidData, "did not get mappings field")),
  }

  Ok(packets)
}

fn parse_int(v: &str) -> Option<IntType> {
  match v {
    "i8" => Some(IntType::I8),
    "u8" => Some(IntType::U8),
    "u16" => Some(IntType::U16),
    "i16" => Some(IntType::I16),
    "i32" => Some(IntType::I32),
    "i64" => Some(IntType::I64),
    "varint" => Some(IntType::VarInt),
    "optvarint" => Some(IntType::OptVarInt),
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
    json::CountType::Typed(v) => {
      CountType::Typed(parse_int(&v).unwrap_or_else(|| panic!("could not parse int: {}", v)))
    }
    json::CountType::Fixed(v) => CountType::Fixed(v),
    json::CountType::Named(v) => CountType::Named(v),
  }
}

fn parse_type(v: json::Type, types: &HashMap<String, PacketField>) -> PacketField {
  match *v.value {
    json::TypeValue::Direct(n) => match n.as_ref() {
      "native" => PacketField::Native,
      "bool" => PacketField::Bool,
      "UUID" => PacketField::UUID,
      "string" => PacketField::String,
      "position" => PacketField::Position,

      "nbt" => PacketField::NBT,
      "optionalNbt" => PacketField::OptionalNBT,
      "restBuffer" => PacketField::RestBuffer,
      "entityMetadata" => PacketField::RestBuffer,

      v => match parse_int(&v) {
        Some(v) => PacketField::Int(v),
        None => match parse_float(&v) {
          Some(v) => PacketField::Float(v),
          None => PacketField::DefinedType(v.to_string()),
        },
      },
    },
    json::TypeValue::Buffer(v) => PacketField::Buffer(parse_count(v.count)),
    json::TypeValue::Array(v) => {
      PacketField::Array { count: parse_count(v.count), value: Box::new(parse_type(v.ty, types)) }
    }
    json::TypeValue::Switch(v) => {
      let mut fields = HashMap::new();
      for (k, v) in v.fields {
        fields.insert(k, parse_type(v, types));
      }
      PacketField::Switch { compare_to: v.compare_to, fields }
    }
    json::TypeValue::Option(v) => PacketField::Option(Box::new(parse_type(v, types))),
    json::TypeValue::Container(v) => {
      let mut fields = HashMap::new();
      for c in v {
        fields.insert(
          c.name.clone().unwrap_or_else(|| "unnamed".into()).to_case(Case::Snake),
          parse_type(c.ty, types),
        );
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
          v,
          u32::from_str_radix(&k[2..], 16)
            .unwrap_or_else(|e| panic!("could not parse packet id: {}, err: {}", k, e)),
        );
      }
      PacketField::Mappings(mappings)
    }
    json::TypeValue::EntityMetadataLoop(_) => PacketField::Native,
    json::TypeValue::TopBitSetTerminatedArray(_) => PacketField::Native,
    json::TypeValue::Custom(v) => {
      if v.len() != 1 {
        panic!("invalid type. got: {:?}", v);
      }
      let k = v.keys().next().unwrap();
      let v = v[k].clone();
      match k.as_ref() {
        "compareTo" => PacketField::CompareTo(v),
        _ => panic!("invalid type. got: {} {}", k, v),
      }
    }
  }
}
