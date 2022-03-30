use bb_common::math::Vec3;
use num_derive::{FromPrimitive, ToPrimitive};
use std::{error::Error, fmt, str::FromStr};

/*
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Type {
  AreaEffectCloud,
  ArmorStand,
  Arrow,
  Bat,
  Bee,
  Blaze,
  Boat,
  Cat,
  CaveSpider,
  Chicken,
  Cod,
  Cow,
  Creeper,
  Dolphin,
  Donkey,
  DragonFireball,
  Drowned,
  ElderGuardian,
  EndCrystal,
  EnderDragon,
  Enderman,
  Endermite,
  Evoker,
  EvokerFangs,
  ExperienceOrb,
  EyeOfEnder,
  FallingBlock,
  FireworkRocket,
  Fox,
  Ghast,
  Giant,
  Guardian,
  Hoglin,
  Horse,
  Husk,
  Illusioner,
  IronGolem,
  Item,
  ItemFrame,
  Fireball,
  LeashKnot,
  LightningBolt,
  Llama,
  LlamaSpit,
  MagmaCube,
  Minecart,
  ChestMinecart,
  CommandBlockMinecart,
  FurnaceMinecart,
  HopperMinecart,
  SpawnerMinecart,
  TntMinecart,
  Mule,
  Mooshroom,
  Ocelot,
  Painting,
  Panda,
  Parrot,
  Phantom,
  Pig,
  Piglin,
  PiglinBrute,
  Pillager,
  PolarBear,
  Tnt,
  Pufferfish,
  Rabbit,
  Ravager,
  Salmon,
  Sheep,
  Shulker,
  ShulkerBullet,
  Silverfish,
  Skeleton,
  SkeletonHorse,
  Slime,
  SmallFireball,
  SnowGolem,
  Snowball,
  SpectralArrow,
  Spider,
  Squid,
  Stray,
  Strider,
  Egg,
  EnderPearl,
  ExperienceBottle,
  Potion,
  Trident,
  TraderLlama,
  TropicalFish,
  Turtle,
  Vex,
  Villager,
  Vindicator,
  WanderingTrader,
  Witch,
  Wither,
  WitherSkeleton,
  WitherSkull,
  Wolf,
  Zoglin,
  Zombie,
  ZombieHorse,
  ZombieVillager,
  ZombifiedPiglin,
  Player,
  FishingBobber,
  None,
}
*/

// Creates the Type enum, and the generate_entities function.
include!(concat!(env!("OUT_DIR"), "/entity/ty.rs"));

/// Any data specific to an entity.
#[derive(Debug)]
pub struct Data {
  ty:     Type,
  id:     u32,
  name:   &'static str,
  width:  f32,
  height: f32,
}

impl Data {
  /// Returns the name of this entity.
  pub fn name(&self) -> &str { &self.name }

  /// Returns the size of the hitbox of this entity.
  pub fn size(&self) -> Vec3 { Vec3::new(self.width as f64, self.height as f64, self.width as f64) }
}

impl Type {
  /// Returns the kind as a u32. Should only be used to index into the
  /// converter's internal table of entity types.
  pub fn id(self) -> u32 { num::ToPrimitive::to_u32(&self).unwrap() }
  /// Returns the entity with the given id. If the id is invalid, this returns
  /// `None`. This differes from items and blocks, as those both have defaults
  /// (air). There is no 'air' like entity, so we need to return an Option here.
  pub fn from_u32(v: u32) -> Option<Type> { num::FromPrimitive::from_u32(v) }
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
  /// Special entities (paintings, exp orbs, and players) will return `true` if
  /// they are living, even though they should all be spawned using special
  /// packets.
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

      Self::Axolotl => true,
      Self::GlowItemFrame => false,
      Self::GlowSquid => true,
      Self::Goat => true,
      Self::Marker => false,
    }
  }
}
