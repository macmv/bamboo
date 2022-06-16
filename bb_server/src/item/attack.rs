//! Implements [`Stack::attack_damage`]

use super::{Stack, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weapon {
  ty:    WeaponType,
  grade: WeaponGrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponType {
  Sword,
  Pickaxe,
  Axe,
  Shovel,
  Hoe,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponGrade {
  Wood,
  Stone,
  Iron,
  Gold,
  Diamond,
}

impl Type {
  /// If this is a weapon, this returns the weapon type and weapon grade. This
  /// only applies to melee weapons, which will deal extra damage. For example,
  /// bows do not count as a weapon here.
  pub fn weapon(&self) -> Option<Weapon> {
    let (ty, grade) = match self {
      Type::WoodenSword => (WeaponType::Sword, WeaponGrade::Wood),
      Type::StoneSword => (WeaponType::Sword, WeaponGrade::Stone),
      Type::IronSword => (WeaponType::Sword, WeaponGrade::Iron),
      Type::GoldenSword => (WeaponType::Sword, WeaponGrade::Gold),
      Type::DiamondSword => (WeaponType::Sword, WeaponGrade::Diamond),

      Type::WoodenPickaxe => (WeaponType::Pickaxe, WeaponGrade::Wood),
      Type::StonePickaxe => (WeaponType::Pickaxe, WeaponGrade::Stone),
      Type::IronPickaxe => (WeaponType::Pickaxe, WeaponGrade::Iron),
      Type::GoldenPickaxe => (WeaponType::Pickaxe, WeaponGrade::Gold),
      Type::DiamondPickaxe => (WeaponType::Pickaxe, WeaponGrade::Diamond),

      Type::WoodenAxe => (WeaponType::Axe, WeaponGrade::Wood),
      Type::StoneAxe => (WeaponType::Axe, WeaponGrade::Stone),
      Type::IronAxe => (WeaponType::Axe, WeaponGrade::Iron),
      Type::GoldenAxe => (WeaponType::Axe, WeaponGrade::Gold),
      Type::DiamondAxe => (WeaponType::Axe, WeaponGrade::Diamond),

      Type::WoodenShovel => (WeaponType::Shovel, WeaponGrade::Wood),
      Type::StoneShovel => (WeaponType::Shovel, WeaponGrade::Stone),
      Type::IronShovel => (WeaponType::Shovel, WeaponGrade::Iron),
      Type::GoldenShovel => (WeaponType::Shovel, WeaponGrade::Gold),
      Type::DiamondShovel => (WeaponType::Shovel, WeaponGrade::Diamond),

      Type::WoodenHoe => (WeaponType::Hoe, WeaponGrade::Wood),
      Type::StoneHoe => (WeaponType::Hoe, WeaponGrade::Stone),
      Type::IronHoe => (WeaponType::Hoe, WeaponGrade::Iron),
      Type::GoldenHoe => (WeaponType::Hoe, WeaponGrade::Gold),
      Type::DiamondHoe => (WeaponType::Hoe, WeaponGrade::Diamond),
      _ => return None,
    };
    Some(Weapon { ty, grade })
  }
}

impl Weapon {
  pub fn base_damage(&self) -> f32 {
    if self.ty == WeaponType::Sword {
      self.grade.base_damage()
    } else {
      0.0
    }
  }
}

impl WeaponGrade {
  pub fn base_damage(&self) -> f32 {
    match self {
      WeaponGrade::Wood => 0.0,
      WeaponGrade::Stone => 1.0,
      WeaponGrade::Iron => 2.0,
      WeaponGrade::Gold => 0.0,
      WeaponGrade::Diamond => 3.0,
    }
  }
}

impl Stack {
  /// Using the item type, the block being mined, and the efficiency of this
  /// item stack, this returns the base speed to mine a block of the given type.
  pub fn attack_damage(&self) -> f32 {
    if let Some(weapon) = self.item().weapon() {
      weapon.base_damage() + 4.0
    } else {
      1.0
    }
  }
}
