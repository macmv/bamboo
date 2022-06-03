use std::{collections::HashMap, error::Error, fmt, str::FromStr};

mod material;
mod prop;

pub use material::Material;
pub use prop::{Prop, PropKind, PropValue};

const STATE_PROPS_LEN: usize = 8;

/// A single block type. This is different from a block kind, which is more
/// general. For example, there is one block kind for oak stairs. However, there
/// are 32 types for an oak stair, based on it's state (rotation, in this case).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Type<'a> {
  pub(super) kind: Kind,
  state:           u32,
  props:           &'a [Prop],
  state_props:     [u32; STATE_PROPS_LEN],
}

impl Kind {
  pub fn data(&self) -> Data {
    let data = unsafe { Box::from_raw(bb_ffi::bb_block_data_for_kind(self.id())) };
    let res = Data {
      kind:          Kind::from_id(data.kind).unwrap(),
      name:          data.name.into_string(),
      material:      Material::Air,
      hardness:      data.hardness,
      resistance:    data.resistance,
      drops:         vec![],
      transparent:   data.transparent.as_bool(),
      filter_light:  data.filter_light,
      emit_light:    data.emit_light,
      state:         data.state,
      bounding_box:  BoundingBoxKind::Empty,
      tags:          vec![],
      props:         vec![],
      default_props: vec![],
    };
    log::info!("{res:#?}");
    res
  }
}

impl Type<'_> {
  /// Returns the type for air.
  pub fn air() -> Type<'static> {
    Type {
      kind:        Kind::Air,
      state:       0,
      props:       &[],
      state_props: [0; STATE_PROPS_LEN],
    }
  }
  /// Returns the block kind that this state comes from.
  pub fn kind(&self) -> Kind { self.kind }
  /// Gets the block id of this type. This id is for the latest version of the
  /// game.
  pub fn id(&self) -> u32 {
    let mut id = 0;
    for (p, sid) in self.props.iter().zip(self.state_props) {
      id *= p.len() as u32;
      id += sid;
    }
    self.state + id
  }
  pub fn prop(&self, name: &str) -> PropValue<'_> {
    let mut idx = None;
    for (i, p) in self.props.iter().enumerate() {
      if p.name == name {
        idx = Some(i);
        break;
      }
    }
    if let Some(idx) = idx {
      let state = self.state_props[idx];
      match self.props[idx].kind {
        PropKind::Bool => match state {
          0 => PropValue::Bool(true),
          _ => PropValue::Bool(false),
        },
        PropKind::Enum(values) => PropValue::Enum(values[state as usize]),
        PropKind::Int { min, max } => PropValue::Int((state + min).min(max)),
      }
    } else {
      panic!("no such property {}, valid properties are {:?}", name, self.props);
    }
  }
  pub fn set_prop<'a>(&mut self, name: &str, val: impl Into<PropValue<'a>>) {
    let mut idx = None;
    for (i, p) in self.props.iter().enumerate() {
      if p.name == name {
        idx = Some(i);
        break;
      }
    }
    if let Some(idx) = idx {
      let val = val.into();
      if val.is(&self.props[idx].kind) {
        self.state_props[idx] = val.id(&self.props[idx].kind);
      } else {
        panic!(
          "the given property {:?} is not compatible with property {:?}",
          val, self.props[idx]
        );
      }
    } else {
      panic!("no such property {}, valid properties are {:?}", name, self.props);
    }
  }
  pub fn with_prop<'a>(mut self, name: &str, val: impl Into<PropValue<'a>>) -> Self {
    self.set_prop(name, val);
    self
  }

  pub fn prop_at(&self, name: &str) -> Option<&Prop> {
    self.props.iter().find(|prop| prop.name == name)
  }
  pub fn props(&self) -> HashMap<String, String> {
    self
      .props
      .iter()
      .enumerate()
      .map(|(i, prop)| (prop.name.into(), prop.kind.name_at(self.state_props[i])))
      .collect()
  }
}
impl fmt::Display for Type<'_> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.kind().to_str())?;
    let mut all_props: Vec<_> = self.props().into_iter().collect();
    all_props.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    if !all_props.is_empty() {
      write!(f, "[")?;
      for (i, (key, val)) in all_props.iter().enumerate() {
        write!(f, "{key}={val}")?;
        if i != all_props.len() - 1 {
          write!(f, ",")?;
        }
      }
      write!(f, "]")?;
    }
    Ok(())
  }
}

#[derive(Debug)]
pub struct InvalidBlock(String);

impl fmt::Display for InvalidBlock {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid block name: {}", self.0)
  }
}

impl Error for InvalidBlock {}

include!(concat!(env!("OUT_DIR"), "/block/ty.rs"));

/// A kind of bounding box. This is from prismarine data. It is not very
/// helpful, and will be replaced when I have a better data source.
#[derive(Debug)]
#[non_exhaustive]
pub enum BoundingBoxKind {
  Empty,
  Block,
}

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  /// The kind for this data.
  pub kind:         Kind,
  /// The name of this block. This is something like `grass_block`.
  pub name:         String,
  /// The material used to make this block. This controls things like map color,
  /// sound, what tool breaks the block, etc. Prismarine doesn't have a very
  /// good material value, so this needs to be updated to more complete data.
  pub material:     Material,
  /// Amount of time it takes to break this block.
  pub hardness:     f32,
  /// How difficult this is to break with an explosion.
  pub resistance:   f32,
  /// A list of item ids this block can drop.
  pub drops:        Vec<ItemDrop>,
  /// If this is true, then clients can (at least partially) see through this
  /// block.
  pub transparent:  bool,
  /// This is how much light this block removes. A value of 15 means it blocks
  /// all light, and a value of 0 means it blocks no light.
  pub filter_light: u8,
  /// The amount of light this block emits (0-15).
  pub emit_light:   u8,
  /// The kind of bounding box this block has.
  pub bounding_box: BoundingBoxKind,

  /// The latest version state id. This is the lowest possible state for this
  /// block. It is used to offset the state calculation for properties.
  pub state:     u32,
  /// A list of vanilla tags for this block. Plugins should be able to add tags
  /// in the future. These tags don't include `minecraft:` at the start.
  pub tags:      Vec<String>,
  /// All the properties on this block. These are stored so that it is easy to
  /// convert a single property on a block.
  props:         Vec<Prop>,
  /// The default type. Each value is an index into that property.
  default_props: Vec<PropValue<'static>>,
}

/// A possible item drop for a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDrop {
  pub item: String,
  pub min:  i32,
  pub max:  i32,
}
