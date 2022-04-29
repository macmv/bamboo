use super::{
  super::{Data, Type},
  Behavior,
};
use bb_common::{math::Pos, util::Face};

pub struct LogBehavior;
impl Behavior for LogBehavior {
  fn place(&self, data: &Data, _: Pos, face: Face) -> Type {
    data.default_type().with_prop(
      "axis",
      match face {
        Face::West | Face::East => "X",
        Face::Top | Face::Bottom => "Y",
        Face::North | Face::South => "Z",
      },
    )
  }
}
