use super::{
  super::{Kind, Type, TypeConverter},
  Behavior,
};
use bb_common::{math::Pos, util::Face};

pub struct LogBehavior;
impl Behavior for LogBehavior {
  fn place(&self, conv: &TypeConverter, kind: Kind, _: Pos, face: Face) -> Type {
    conv.get(kind).default_type().with_prop(
      "axis",
      match face {
        Face::West | Face::East => "x",
        Face::Top | Face::Bottom => "y",
        Face::North | Face::South => "z",
      },
    )
  }
}
