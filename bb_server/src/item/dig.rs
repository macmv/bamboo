//! Implements [`Stack::mining_speed`]

use super::{Stack, Type};
use crate::{block, block::Material};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tool {
  ty:    ToolType,
  grade: ToolGrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
  Sword,
  Pickaxe,
  Axe,
  Shovel,
  Hoe,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolGrade {
  Wood,
  Stone,
  Iron,
  Gold,
  Diamond,
}

impl Type {
  /// If this is a tool, this returns the tool type and tool grade.
  pub fn tool(&self) -> Option<Tool> {
    let (ty, grade) = match self {
      Type::WoodenSword => (ToolType::Sword, ToolGrade::Wood),
      Type::StoneSword => (ToolType::Sword, ToolGrade::Stone),
      Type::IronSword => (ToolType::Sword, ToolGrade::Iron),
      Type::GoldenSword => (ToolType::Sword, ToolGrade::Gold),
      Type::DiamondSword => (ToolType::Sword, ToolGrade::Diamond),

      Type::WoodenPickaxe => (ToolType::Pickaxe, ToolGrade::Wood),
      Type::StonePickaxe => (ToolType::Pickaxe, ToolGrade::Stone),
      Type::IronPickaxe => (ToolType::Pickaxe, ToolGrade::Iron),
      Type::GoldenPickaxe => (ToolType::Pickaxe, ToolGrade::Gold),
      Type::DiamondPickaxe => (ToolType::Pickaxe, ToolGrade::Diamond),

      Type::WoodenAxe => (ToolType::Axe, ToolGrade::Wood),
      Type::StoneAxe => (ToolType::Axe, ToolGrade::Stone),
      Type::IronAxe => (ToolType::Axe, ToolGrade::Iron),
      Type::GoldenAxe => (ToolType::Axe, ToolGrade::Gold),
      Type::DiamondAxe => (ToolType::Axe, ToolGrade::Diamond),

      Type::WoodenShovel => (ToolType::Shovel, ToolGrade::Wood),
      Type::StoneShovel => (ToolType::Shovel, ToolGrade::Stone),
      Type::IronShovel => (ToolType::Shovel, ToolGrade::Iron),
      Type::GoldenShovel => (ToolType::Shovel, ToolGrade::Gold),
      Type::DiamondShovel => (ToolType::Shovel, ToolGrade::Diamond),

      Type::WoodenHoe => (ToolType::Hoe, ToolGrade::Wood),
      Type::StoneHoe => (ToolType::Hoe, ToolGrade::Stone),
      Type::IronHoe => (ToolType::Hoe, ToolGrade::Iron),
      Type::GoldenHoe => (ToolType::Hoe, ToolGrade::Gold),
      Type::DiamondHoe => (ToolType::Hoe, ToolGrade::Diamond),
      _ => return None,
    };
    Some(Tool { ty, grade })
  }
}

impl Tool {
  pub fn does_mine(&self, block: &block::Data) -> bool {
    if !self.ty.does_mine(block.material) {
      return false;
    }
    let required_grade = match block.kind {
      block::Kind::IronBlock
      | block::Kind::IronOre
      | block::Kind::LapisBlock
      | block::Kind::LapisOre => ToolGrade::Stone,
      block::Kind::DiamondBlock
      | block::Kind::DiamondOre
      | block::Kind::EmeraldBlock
      | block::Kind::EmeraldOre
      | block::Kind::GoldBlock
      | block::Kind::GoldOre
      | block::Kind::RedstoneOre => ToolGrade::Iron,
      block::Kind::Obsidian => ToolGrade::Diamond,

      _ => ToolGrade::Wood,
    };
    self.grade.mining_level() >= required_grade.mining_level()
  }
}

impl ToolType {
  pub fn does_mine(&self, material: Material) -> bool {
    match self {
      Self::Sword => matches!(material, Material::Cobweb),
      Self::Pickaxe => matches!(material, Material::Metal | Material::Stone),
      Self::Axe => matches!(material, Material::Wood),
      Self::Shovel => matches!(material, Material::Soil),
      Self::Hoe => false,
    }
  }
}

impl ToolGrade {
  pub fn mining_level(&self) -> u8 {
    match self {
      Self::Wood | Self::Gold => 0,
      Self::Stone => 1,
      Self::Iron => 2,
      Self::Diamond => 3,
    }
  }
  pub fn base_speed(&self) -> f64 {
    match self {
      ToolGrade::Wood => 2.0,
      ToolGrade::Stone => 4.0,
      ToolGrade::Iron => 6.0,
      ToolGrade::Gold => 8.0,
      ToolGrade::Diamond => 12.0,
    }
  }
}

impl Stack {
  /// Using the item type, the block being mined, and the efficiency of this
  /// item stack, this returns the base speed to mine a block of the given type.
  pub fn mining_speed(&self, block: &block::Data) -> f64 {
    let speed = if !block.material.requires_tool() {
      // doesn't require tool
      1.0 / 30.0
    } else {
      // requires tool
      if let Some(tool) = self.item().tool() {
        if tool.does_mine(block) {
          tool.grade.base_speed() / 30.0
        } else {
          // the tool we have isn't correct
          1.0 / 100.0
        }
      } else {
        // we don't have a tool
        1.0 / 100.0
      }
    };
    speed / block.hardness as f64
  }
}
