use super::{Behavior, Entity};

pub struct SnowballBehavior;
impl Behavior for SnowballBehavior {
  fn tick(&self, ent: &Entity) {
    info!("SNOWBALL TICK");
  }
}
