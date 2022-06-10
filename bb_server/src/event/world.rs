use super::Events;
use crate::world::{World, WorldManager};

impl WorldManager {
  pub fn events(&self) -> Events { Events { wm: self } }
}
impl World {
  pub fn events(&self) -> Events { Events { wm: self.world_manager() } }
}
