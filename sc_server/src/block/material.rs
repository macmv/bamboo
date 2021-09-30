//! This deals with materials. It is similar to 1.8, but because of prismarine
//! data, is greatly simplified. This will later be extended to have all the
//! materials that I think make sense (so around half the ones in 1.8). I will
//! extend this once I have a custom data generator setup that can extract all
//! that I need from the various Minecraft versions.
//!
//! Here are the 1.8 materials:
//! - air = MaterialTransparent(air);
//! - grass = Material(grass);
//! - ground = Material(dirt);
//! - wood = (Material(wood)).setBurning();
//! - rock = (Material(stone)).setRequiresTool();
//! - iron = (Material(iron)).setRequiresTool();
//! - anvil = (Material(iron)).setRequiresTool().setImmovableMobility();
//! - water = (MaterialLiquid(water)).setNoPushMobility();
//! - lava = (MaterialLiquid(tnt)).setNoPushMobility();
//! - leaves = (Material(foliage)).setBurning().setTranslucent().
//!   setNoPushMobility();
//! - plants = (MaterialLogic(foliage)).setNoPushMobility();
//! - vine = (MaterialLogic(foliage)).setBurning().setNoPushMobility().
//!   setReplaceable();
//! - sponge = Material(yellow);
//! - cloth = (Material(cloth)).setBurning();
//! - fire = (MaterialTransparent(air)).setNoPushMobility();
//! - sand = Material(sand);
//! - circuits = (MaterialLogic(air)).setNoPushMobility();
//! - carpet = (MaterialLogic(cloth)).setBurning();
//! - glass = (Material(air)).setTranslucent().setAdventureModeExempt();
//! - redstoneLight = (Material(air)).setAdventureModeExempt();
//! - tnt = (Material(tnt)).setBurning().setTranslucent();
//! - coral = (Material(foliage)).setNoPushMobility();
//! - ice = (Material(ice)).setTranslucent().setAdventureModeExempt();
//! - packedIce = (Material(ice)).setAdventureModeExempt();
//! - snow = (MaterialLogic(snow)).setReplaceable().setTranslucent().
//!   setRequiresTool().setNoPushMobility();
//! - craftedSnow = (Material(snow)).setRequiresTool();
//! - cactus = (Material(foliage)).setTranslucent().setNoPushMobility();
//! - clay = Material(clay);
//! - gourd = (Material(foliage)).setNoPushMobility();
//! - dragonEgg = (Material(foliage)).setNoPushMobility();
//! - portal = (MaterialPortal(air)).setImmovableMobility();
//! - cake = (Material(air)).setNoPushMobility();
//! - web = (Material(cloth)).setRequiresTool().setNoPushMobility();
//! - piston = (Material(stone)).setImmovableMobility();
//! - barrier = (Material(air)).setRequiresTool().setImmovableMobility();

/// A block material. In vanilla, there are around 50 of these. However, the
/// prismarine data limits it to these options. This will be updated in the
/// future.
#[derive(Debug)]
#[non_exhaustive]
pub enum Material {
  Air,
  Rock,
  Dirt,
  Plant,
  Wood,
  Web,
  Wool,
  Unknown,
}

impl Material {
  pub fn is_replaceable(&self) -> bool {
    matches!(self, Material::Plant)
  }
}
