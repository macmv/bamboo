use num_derive::{FromPrimitive, ToPrimitive};
use std::{error::Error, fmt, str::FromStr};

// Creates the Type enum, and the generate_entities function.
include!(concat!(env!("OUT_DIR"), "/entity/ty.rs"));

/// Any data specific to an entity.
#[derive(Debug)]
pub struct Data {
  display_name: &'static str,
  width:        f32,
  height:       f32,
}

impl Data {
  pub fn display_name(&self) -> &str {
    &self.display_name
  }
}

impl Type {
  /// Returns the kind as a u32. Should only be used to index into the
  /// converter's internal table of entity types.
  pub fn to_u32(self) -> u32 {
    num::ToPrimitive::to_u32(&self).unwrap()
  }
  /// Returns the item with the given id. If the id is invalid, this returns
  /// `Type::Air`.
  pub fn from_u32(v: u32) -> Type {
    num::FromPrimitive::from_u32(v).unwrap_or(Type::None)
  }
}

#[derive(Debug)]
pub struct InvalidEntity(String);

impl fmt::Display for InvalidEntity {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid entity name: {}", self.0)
  }
}

impl Error for InvalidEntity {}

impl Type {
  /// If this is true, the entity has health, a head yaw, and does some more
  /// fancy nonsense. This is mostly used when spawning entities, as non-living
  /// entities have a different packet.
  ///
  /// Special entities (paintings, exp orbs, and players) will return if they
  /// are living, even though they should all be spawned using special packets.
  pub fn is_living(&self) -> bool {
    match self {
      Self::AreaEffectCloud => false,
      Self::ArmorStand => true,
      Self::Arrow => false,
      Self::Bat => true,
      Self::Bee => true,
      Self::Blaze => true,
      Self::Boat => false,
      Self::Cat => true,
      Self::CaveSpider => true,
      Self::Chicken => true,
      Self::Cod => true,
      Self::Cow => true,
      Self::Creeper => true,
      Self::Dolphin => true,
      Self::Donkey => true,
      Self::DragonFireball => false,
      Self::Drowned => true,
      Self::ElderGuardian => true,
      Self::EndCrystal => false,
      Self::EnderDragon => true,
      Self::Enderman => true,
      Self::Endermite => true,
      Self::Evoker => true,
      Self::EvokerFangs => false,
      Self::ExperienceOrb => false,
      Self::EyeOfEnder => false,
      Self::FallingBlock => false,
      Self::FireworkRocket => false,
      Self::Fox => true,
      Self::Ghast => true,
      Self::Giant => true,
      Self::Guardian => true,
      Self::Hoglin => true,
      Self::Horse => true,
      Self::Husk => true,
      Self::Illusioner => true,
      Self::IronGolem => true,
      Self::Item => false,
      Self::ItemFrame => false,
      Self::Fireball => false,
      Self::LeashKnot => false,
      Self::LightningBolt => false,
      Self::Llama => true,
      Self::LlamaSpit => false,
      Self::MagmaCube => true,
      Self::Minecart => false,
      Self::ChestMinecart => false,
      Self::CommandBlockMinecart => false,
      Self::FurnaceMinecart => false,
      Self::HopperMinecart => false,
      Self::SpawnerMinecart => false,
      Self::TntMinecart => false,
      Self::Mule => true,
      Self::Mooshroom => true,
      Self::Ocelot => true,
      Self::Painting => false,
      Self::Panda => true,
      Self::Parrot => true,
      Self::Phantom => true,
      Self::Pig => true,
      Self::Piglin => true,
      Self::PiglinBrute => true,
      Self::Pillager => true,
      Self::PolarBear => true,
      Self::Tnt => false,
      Self::Pufferfish => true,
      Self::Rabbit => true,
      Self::Ravager => true,
      Self::Salmon => true,
      Self::Sheep => true,
      Self::Shulker => true,
      Self::ShulkerBullet => false,
      Self::Silverfish => true,
      Self::Skeleton => true,
      Self::SkeletonHorse => true,
      Self::Slime => true,
      Self::SmallFireball => false,
      Self::SnowGolem => true,
      Self::Snowball => false,
      Self::SpectralArrow => false,
      Self::Spider => true,
      Self::Squid => true,
      Self::Stray => true,
      Self::Strider => true,
      Self::Egg => false,
      Self::EnderPearl => false,
      Self::ExperienceBottle => false,
      Self::Potion => false,
      Self::Trident => false,
      Self::TraderLlama => true,
      Self::TropicalFish => true,
      Self::Turtle => true,
      Self::Vex => true,
      Self::Villager => true,
      Self::Vindicator => true,
      Self::WanderingTrader => true,
      Self::Witch => true,
      Self::Wither => true,
      Self::WitherSkeleton => true,
      Self::WitherSkull => false,
      Self::Wolf => true,
      Self::Zoglin => true,
      Self::Zombie => true,
      Self::ZombieHorse => true,
      Self::ZombieVillager => true,
      Self::ZombifiedPiglin => true,
      Self::Player => true,
      Self::FishingBobber => false,
      Self::None => false,
    }
  }
}
