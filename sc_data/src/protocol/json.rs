//! This is the module that handles parsing the prismarine protocol json data.
//! This uses serde, and has a bunch of structs that match up with the json.
//! This has `#[allow(dead_code)]` used every now and then, as this should be
//! read from json, and we want to error out of the json doesn't contain certain
//! keys.

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_derive::Deserialize;
use std::collections::HashMap;

pub(crate) type TypeMap = HashMap<String, Type>;

#[derive(Debug, Clone)]
pub(crate) struct Type {
  pub(crate) kind:  String,
  pub(crate) value: Box<TypeValue>,
}

impl<'de> Deserialize<'de> for Type {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct Arr;

    macro_rules! match_values {
      ($self:expr, $k:expr, $s:expr, [$($name:expr, $kind:ident),*,]) => {
        match $k {
          $(
            $name => TypeValue::$kind($s.next_element()?.ok_or_else(|| de::Error::invalid_length(1, $self))?),
          )*
          _ => TypeValue::Custom($s.next_element()?.ok_or_else(|| de::Error::invalid_length(1, $self))?),
        }
      };
    }

    impl<'de> Visitor<'de> for Arr {
      type Value = Type;

      fn expecting(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "a protocol type field")
      }

      fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
      where
        E: de::Error,
      {
        Ok(Type { kind: "direct".into(), value: Box::new(TypeValue::Direct(s.into())) })
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
            BitField,
            "topBitSetTerminatedArray",
            TopBitSetTerminatedArray,
            "entityMetadataLoop",
            EntityMetadataLoop,
          ]
        );
        Ok(Type { kind: kind.into(), value: Box::new(value) })
      }
    }

    deserializer.deserialize_any(Arr)
  }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum TypeValue {
  // Just a string type (will be something like "varint")
  Direct(String),
  // A container. This is a list of objects
  Container(Vec<Container>),
  // This is a list of fields, with a count type
  Array(Array),
  // This is a list of mappings. TODO: Learn more
  Mapper(Mapper),
  // This is a list of conditionals and values. TODO: Learn more
  Switch(Switch),
  // This is a byte array, with a length type
  Buffer(Buffer),
  // This is a value that may or may not exist
  Option(Type),
  // This is a value that may or may not exist
  BitField(Vec<BitField>),
  // This is an array of elements, where each element is a contianer. The first element in that
  // container must be a number type. The highest bit in that number will be set if another entry
  // continues. Otherwise, the given container is the last item in the array.
  //
  // Minecraft dude. Why?
  TopBitSetTerminatedArray(TopBitSetTerminatedArray),
  // Entity metadata
  EntityMetadataLoop(EntityMetadataLoop),
  Custom(HashMap<String, String>),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct TopBitSetTerminatedArray {
  pub(crate) ty: Type,
}

impl<'de> Deserialize<'de> for TopBitSetTerminatedArray {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct Inner;

    impl<'de> Visitor<'de> for Inner {
      type Value = TopBitSetTerminatedArray;

      fn expecting(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "a protocol type field")
      }

      fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
      where
        V: MapAccess<'de>,
      {
        let mut ty = None;
        while let Some(key) = map.next_key()? {
          match key {
            "type" => {
              if ty.is_some() {
                return Err(de::Error::duplicate_field("type"));
              }
              ty = Some(map.next_value()?);
            }
            v => return Err(de::Error::unknown_field(v, &["type"])),
          }
        }
        let ty = ty.ok_or_else(|| de::Error::missing_field("type"))?;
        Ok(TopBitSetTerminatedArray { ty })
      }
    }

    deserializer.deserialize_any(Inner)
  }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct EntityMetadataLoop {
  #[serde(alias = "endVal")]
  pub(crate) end: u32,
  #[serde(alias = "type")]
  pub(crate) ty:  Type,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct BitField {
  pub(crate) name:   String,
  pub(crate) size:   u32,
  pub(crate) signed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Container {
  // If there is no name, then this is an anonymous container
  pub(crate) name: Option<String>,
  pub(crate) ty:   Type,
}

impl<'de> Deserialize<'de> for Container {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct Inner;

    impl<'de> Visitor<'de> for Inner {
      type Value = Container;

      fn expecting(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "a protocol type field")
      }

      fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
      where
        V: MapAccess<'de>,
      {
        let mut name = None;
        let mut anon: Option<bool> = None;
        let mut ty = None;
        while let Some(key) = map.next_key()? {
          match key {
            "anon" => {
              if anon.is_some() || name.is_some() {
                return Err(de::Error::duplicate_field("anon"));
              }
              anon = Some(map.next_value()?);
            }
            "name" => {
              if anon.is_some() || name.is_some() {
                return Err(de::Error::duplicate_field("name"));
              }
              name = Some(map.next_value()?);
            }
            "type" => {
              if ty.is_some() {
                return Err(de::Error::duplicate_field("type"));
              }
              ty = Some(map.next_value()?);
            }
            v => return Err(de::Error::unknown_field(v, &["anon", "name", "type"])),
          }
        }
        if anon.is_none() && name.is_none() {
          return Err(de::Error::missing_field("name"));
        }
        let ty = ty.ok_or_else(|| de::Error::missing_field("type"))?;
        Ok(Container { name, ty })
      }
    }

    deserializer.deserialize_any(Inner)
  }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Buffer {
  // The type that the length is in
  #[serde(alias = "countType")]
  pub(crate) count: CountType,
}

// If count_type is none, then it is a fixed length array of length count. If
// count is none, then this is an array prefixed with a value of the given type.
// If both are none, this is invalid.
#[derive(Debug, Clone)]
pub(crate) struct Array {
  pub(crate) count: CountType,
  pub(crate) ty:    Type,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub(crate) enum CountType {
  // The array will be prefixed with this field.
  // This will be deserialized from count_type, so we don't want serde to deserialize to this. See
  // [`Array`] for more.
  #[serde(skip)]
  Typed(String),
  // A hardocded count
  Fixed(u32),
  // Another protocol field should be used as the count
  Named(String),
}

impl<'de> Deserialize<'de> for Array {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct Inner;

    impl<'de> Visitor<'de> for Inner {
      type Value = Array;

      fn expecting(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "an array object")
      }

      fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
      where
        V: MapAccess<'de>,
      {
        let mut count_type: Option<String> = None;
        let mut count = None;
        let mut ty = None;
        while let Some(key) = map.next_key()? {
          match key {
            "countType" => {
              if count.is_some() || count_type.is_some() {
                return Err(de::Error::duplicate_field("count_type"));
              }
              count_type = Some(map.next_value()?);
            }
            "count" => {
              if count.is_some() || count_type.is_some() {
                return Err(de::Error::duplicate_field("count"));
              }
              count = Some(map.next_value()?);
            }
            "type" => {
              if ty.is_some() {
                return Err(de::Error::duplicate_field("type"));
              }
              ty = Some(map.next_value()?);
            }
            v => return Err(de::Error::unknown_field(v, &["count", "count_type", "type"])),
          }
        }
        let count = match count {
          Some(v) => v,
          None => match count_type {
            Some(v) => CountType::Typed(v),
            None => return Err(de::Error::missing_field("count")),
          },
        };
        let ty = ty.ok_or_else(|| de::Error::missing_field("type"))?;
        Ok(Array { count, ty })
      }
    }

    deserializer.deserialize_any(Inner)
  }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct Mapper {
  pub(crate) mappings: HashMap<String, String>,
  #[serde(alias = "type")]
  pub(crate) ty:       Type,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct Switch {
  // Another field name to be compared with
  #[serde(alias = "compareTo")]
  pub(crate) compare_to: String,
  pub(crate) fields:     HashMap<String, Type>,
  pub(crate) default:    Option<Type>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PacketMap {
  // This is a map of length two arrays
  // The first element is a PacketEntry::Kind, and the second is a PacketEntry::Value.
  pub(crate) types: TypeMap,
}

#[derive(Debug, Deserialize)]
pub struct ClientServerMap {
  #[serde(alias = "toClient")]
  pub(crate) to_client: PacketMap,
  #[serde(alias = "toServer")]
  pub(crate) to_server: PacketMap,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ProtocolVersion {
  pub(crate) types:       HashMap<String, Type>,
  pub(crate) handshaking: ClientServerMap,
  pub(crate) status:      ClientServerMap,
  pub(crate) login:       ClientServerMap,
  pub(crate) play:        ClientServerMap,
}
