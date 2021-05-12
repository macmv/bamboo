use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_derive::Deserialize;
use std::{collections::HashMap, convert::TryInto};

pub type TypeMap = HashMap<String, Type>;

#[derive(Debug)]
pub struct Type {
  pub kind:  String,
  pub value: Box<TypeValue>,
}

impl<'de> Deserialize<'de> for Type {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct Arr;

    macro_rules! match_values {
      ($self:expr, $k:expr, $s:expr, [$($name:expr, $kind:ident),*]) => {
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
            Bitfield
          ]
        );
        Ok(Type { kind: kind.into(), value: Box::new(value) })
      }
    }

    deserializer.deserialize_any(Arr)
  }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TypeValue {
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
  Option(Option<Type>),
  // This is a value that may or may not exist
  Bitfield(Vec<Bitfield>),
  // Custom type
  Custom(HashMap<String, String>),
}

#[derive(Debug, Deserialize)]
pub struct Bitfield {
  pub name:   String,
  pub size:   u32,
  pub signed: bool,
}

#[derive(Debug)]
pub struct Container {
  // If there is no name, then this is an anonymous container
  pub name: Option<String>,
  pub ty:   Type,
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

// If count_type is none, then it is a fixed length array of length count. If
// count is none, then this is an array prefixed with a value of the given type.
// If both are none, this is invalid.
#[derive(Debug)]
pub struct Array {
  pub count: CountType,
  pub ty:    Type,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CountType {
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
        dbg!(&count);
        Ok(Array { count, ty })
      }
    }

    deserializer.deserialize_any(Inner)
  }
}

#[derive(Debug, Deserialize)]
pub struct Object {
  pub name: String,
  #[serde(alias = "type")]
  pub ty:   Type,
}

#[derive(Debug, Deserialize)]
pub struct AnonObject {
  pub anon: bool,
  #[serde(alias = "type")]
  pub ty:   Type,
}

#[derive(Debug, Deserialize)]
pub struct Mapper {
  pub mappings: HashMap<String, String>,
  #[serde(alias = "type")]
  pub ty:       Type,
}

#[derive(Debug, Deserialize)]
pub struct Switch {
  // Another field name to be compared with
  #[serde(alias = "compareTo")]
  pub compare_to: String,
  pub fields:     HashMap<String, Type>,
}

#[derive(Debug, Deserialize)]
pub struct Buffer {
  // The type that the length is in
  #[serde(alias = "countType")]
  pub count_type: String,
}

#[derive(Debug, Deserialize)]
pub struct PacketMap {
  // This is a map of length two arrays
  // The first element is a PacketEntry::Kind, and the second is a PacketEntry::Value.
  pub types: TypeMap,
}

#[derive(Debug, Deserialize)]
pub struct ClientServerMap {
  #[serde(alias = "toClient")]
  pub to_client: PacketMap,
  #[serde(alias = "toServer")]
  pub to_server: PacketMap,
}

#[derive(Debug, Deserialize)]
pub struct ProtocolVersion {
  // types: HashMap<String, TypeDef>,
  pub handshaking: ClientServerMap,
  pub status:      ClientServerMap,
  pub login:       ClientServerMap,
  pub play:        ClientServerMap,
}
