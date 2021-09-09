mod err;
mod token;

pub use err::{ChildError, ErrorKind, ParseError, Result};
pub use token::{Span, Tokenizer, Word};

use super::{Arg, CommandSender, Parser, StringType};
use crate::block;
use common::math::Pos;
use std::{collections::HashMap, fmt::Display, str::FromStr};

pub fn parse_num<T>(w: &Word, min: &Option<T>, max: &Option<T>) -> Result<T>
where
  T: PartialOrd + FromStr + Copy + Display,
{
  let num = w.parse::<T>().map_err(|_| w.expected("a number"))?;
  if let Some(min) = min {
    if num < *min {
      if let Some(max) = max {
        return Err(w.expected(format!("a number between {} and {}", min, max)));
      } else {
        return Err(w.expected(format!("a number above {}", min)));
      }
    }
  }
  if let Some(max) = max {
    if num > *max {
      if let Some(min) = min {
        return Err(w.expected(format!("a number between {} and {}", min, max)));
      } else {
        return Err(w.expected(format!("a number below {}", max)));
      }
    }
  }
  Ok(num)
}

impl Parser {
  pub fn parse<S>(&self, tokens: &mut Tokenizer, sender: &S) -> Result<Arg>
  where
    S: CommandSender,
  {
    match self {
      Self::Bool => {
        let w = tokens.read_spaced_word()?;
        if w == "true" {
          Ok(Arg::Bool(true))
        } else if w == "false" {
          Ok(Arg::Bool(false))
        } else {
          Err(w.invalid())
        }
      }
      Self::Double { min, max } => {
        let w = tokens.read_spaced_text()?;
        let num = parse_num(&w, min, max)?;
        Ok(Arg::Double(num))
      }
      Self::Float { min, max } => {
        let w = tokens.read_spaced_text()?;
        let num = parse_num(&w, min, max)?;
        Ok(Arg::Float(num))
      }
      Self::Int { min, max } => {
        let w = tokens.read_spaced_text()?;
        let num = parse_num(&w, min, max)?;
        Ok(Arg::Int(num))
      }
      Self::BlockPos => {
        if let Some(pos) = sender.block_pos() {
          let mut w = tokens.read_spaced_text()?;
          let x_rel = w.starts_with("~");
          if x_rel {
            w.set_text(w[1..].to_string());
          }
          let x = parse_num(&w, &None, &None)?;
          let mut w = tokens.read_spaced_text()?;
          let y_rel = w.starts_with("~");
          if y_rel {
            w.set_text(w[1..].to_string());
          }
          let y = parse_num(&w, &None, &None)?;
          let mut w = tokens.read_spaced_text()?;
          let z_rel = w.starts_with("~");
          if z_rel {
            w.set_text(w[1..].to_string());
          }
          let z = parse_num(&w, &None, &None)?;

          Ok(Arg::BlockPos(Pos::new(
            if x_rel { pos.x() + x } else { x },
            if y_rel { pos.y() + y } else { y },
            if z_rel { pos.z() + z } else { z },
          )))
        } else {
          let w = tokens.read_spaced_text()?;
          let x = parse_num(&w, &None, &None)?;
          let w = tokens.read_spaced_text()?;
          let y = parse_num(&w, &None, &None)?;
          let w = tokens.read_spaced_text()?;
          let z = parse_num(&w, &None, &None)?;

          Ok(Arg::BlockPos(Pos::new(x, y, z)))
        }
      }
      Self::BlockState => {
        let w = tokens.read_spaced_word()?;
        Ok(Arg::BlockState(
          block::Kind::from_str(&w).map_err(|_| w.invalid())?,
          HashMap::new(),
          None,
        ))
      }
      _ => unimplemented!(),
      /* Self::String(ty) => match ty {
       *   StringType::Word => {
       *     if text.is_empty() {
       *       return Err(ParseError::EOF);
       *     }
       *     let word = parse_word(text);
       *     let len = word.len();
       *     Ok((Arg::String(word), len))
       *   }
       *   StringType::Quotable => {
       *     if text.is_empty() {
       *       return Err(ParseError::EOF);
       *     }
       *     let mut iter = text.chars();
       *     if iter.next().unwrap() == '"' {
       *       let mut escaping = false;
       *       let mut index = 1;
       *       let mut out = String::new();
       *       for c in iter {
       *         if escaping {
       *           if c == '"' || c == '\\' {
       *             out.push(c);
       *           } else {
       *             return Err(ParseError::InvalidText(
       *               text.into(),
       *               "a valid escape character".into(),
       *             ));
       *           }
       *           escaping = false;
       *         } else {
       *           if c == '"' {
       *             break;
       *           } else if c == '\\' {
       *             escaping = true;
       *           } else {
       *             out.push(c);
       *           }
       *         }
       *         index += 1;
       *       }
       *       // Add 1 so that the ending quote is removed
       *       Ok((Arg::String(out), index + 1))
       *     } else {
       *       let mut index = 1;
       *       for c in iter {
       *         if c == ' ' {
       *           break;
       *         }
       *         index += 1;
       *       }
       *       Ok((Arg::String(text[0..index].into()), index))
       *     }
       *   }
       *   StringType::Greedy => Ok((Arg::String(text.into()), text.len())),
       * },
       * Self::Entity { single: _, players: _ } => Ok((Arg::Int(5), 1)),
       * Self::ScoreHolder { multiple: _ } => Ok((Arg::Int(5), 1)),
       * Self::GameProfile => Ok((Arg::Int(5), 1)),
       * Self::BlockPos => {
       *   let sections: Vec<&str> = text.split(' ').collect();
       *   if sections.len() < 3 {
       *     return Err(ParseError::InvalidText(text.into(), "a block position".into()));
       *   }
       *   let x = sections[0]
       *     .parse()
       *     .map_err(|_| ParseError::InvalidText(text.into(), "a valid block
       * position".into()))?;   let y = sections[1]
       *     .parse()
       *     .map_err(|_| ParseError::InvalidText(text.into(), "a valid block
       * position".into()))?;   let z = sections[2]
       *     .parse()
       *     .map_err(|_| ParseError::InvalidText(text.into(), "a valid block
       * position".into()))?;   Ok((
       *     Arg::BlockPos(Pos::new(x, y, z)),
       *     sections[0].len() + sections[1].len() + sections[2].len() + 2,
       *   ))
       * }
       * Self::ColumnPos => Ok((Arg::Int(5), 1)),
       * Self::Vec3 => Ok((Arg::Int(5), 1)),
       * Self::Vec2 => Ok((Arg::Int(5), 1)),
       * Self::BlockState => {
       *   let word = parse_word(text);
       *   Ok((
       *     Arg::BlockState(
       *       block::Kind::from_str(&word)
       *         .map_err(|_| ParseError::InvalidText(text.into(), "a valid block
       * name".into()))?,       HashMap::new(),
       *       None,
       *     ),
       *     word.len(),
       *   ))
       * }
       * Self::BlockPredicate => Ok((Arg::Int(5), 1)),
       * Self::ItemStack => Ok((Arg::Int(5), 1)),
       * Self::ItemPredicate => Ok((Arg::Int(5), 1)),
       * Self::Color => Ok((Arg::Int(5), 1)),
       * Self::Component => Ok((Arg::Int(5), 1)),
       * Self::Message => Ok((Arg::Int(5), 1)),
       * Self::Nbt => Ok((Arg::Int(5), 1)),
       * Self::NbtPath => Ok((Arg::Int(5), 1)),
       * Self::Objective => Ok((Arg::Int(5), 1)),
       * Self::ObjectiveCriteria => Ok((Arg::Int(5), 1)),
       * Self::Operation => Ok((Arg::Int(5), 1)),
       * Self::Particle => Ok((Arg::Int(5), 1)),
       * Self::Rotation => Ok((Arg::Int(5), 1)),
       * Self::Angle => Ok((Arg::Int(5), 1)),
       * Self::ScoreboardSlot => Ok((Arg::Int(5), 1)),
       * Self::Swizzle => Ok((Arg::Int(5), 1)),
       * Self::Team => Ok((Arg::Int(5), 1)),
       * Self::ItemSlot => Ok((Arg::Int(5), 1)),
       * Self::ResourceLocation => Ok((Arg::Int(5), 1)),
       * Self::MobEffect => Ok((Arg::Int(5), 1)),
       * Self::Function => Ok((Arg::Int(5), 1)),
       * Self::EntityAnchor => Ok((Arg::Int(5), 1)),
       * Self::Range { decimals: _bool } => Ok((Arg::Int(5), 1)),
       * Self::IntRange => Ok((Arg::Int(5), 1)),
       * Self::FloatRange => Ok((Arg::Int(5), 1)),
       * Self::ItemEnchantment => Ok((Arg::Int(5), 1)),
       * Self::EntitySummon => Ok((Arg::Int(5), 1)),
       * Self::Dimension => Ok((Arg::Int(5), 1)),
       * Self::Uuid => Ok((Arg::Int(5), 1)),
       * Self::NbtTag => Ok((Arg::Int(5), 1)),
       * Self::NbtCompoundTag => Ok((Arg::Int(5), 1)),
       * Self::Time => Ok((Arg::Int(5), 1)),
       * Self::Modid => Ok((Arg::Int(5), 1)),
       * Self::Enum => Ok((Arg::Int(5), 1)), */
    }
  }

  pub fn desc(&self) -> &'static str {
    match self {
      Self::Bool => "true or false",
      Self::Double { .. } => "a double",
      Self::Float { .. } => "a float",
      Self::Int { .. } => "an int",
      Self::String(ty) => match ty {
        StringType::Word => "a word",
        StringType::Quotable => "a quotable phrase",
        StringType::Greedy => "any text",
      },
      Self::Entity { .. } => "an entity",
      Self::ScoreHolder { .. } => "a score holder",
      Self::GameProfile { .. } => "a username",
      Self::BlockPos { .. } => "a block position",
      Self::ColumnPos => "a block column position",
      Self::Vec3 => "a 3 value vector",
      Self::Vec2 => "a 2 value vector",
      Self::BlockState => "a block id",
      Self::BlockPredicate => "a block id or tag",
      Self::ItemStack => "an item stack",
      Self::ItemPredicate => "an item id or tag",
      Self::Color => "a chat color",
      Self::Component => "a JSON chat message",
      Self::Message => "a message",
      Self::Nbt => "a JSON NBT tag",
      Self::NbtPath => "a path within an NBT tag",
      Self::Objective => "a scoreboard objective",
      Self::ObjectiveCriteria => "a single score criterion",
      Self::Operation => "a scoreboard operator",
      Self::Particle => "a particle id",
      Self::Rotation => "yaw and pitch values",
      Self::Angle => "an angle",
      Self::ScoreboardSlot => "a scoreboard slot (list, sidebar, etc.)",
      Self::Swizzle => "a collection of up to 3 axis (x, xy, zyx, etc.)",
      Self::Team => "a team name",
      Self::ItemSlot => "a name for an inventory slot",
      Self::ResourceLocation => "a resource path (stone, minecraft:dirt, my_mod::foo, etc.)",
      Self::MobEffect => "a potion effect",
      Self::Function => "a function",
      Self::EntityAnchor => "an entity anchor",
      Self::Range { .. } => "a range of numbers (0..5, 2.3..5.6)",
      Self::IntRange => "a range of ints (0..5)",
      Self::FloatRange => "a range of floats (2.3..5.6)",
      Self::ItemEnchantment => "an item enchantment",
      Self::EntitySummon => "an entity id",
      Self::Dimension => "a dimension",
      Self::Uuid => "a UUID",
      Self::NbtTag => "a partial NBT tag",
      Self::NbtCompoundTag => "a complete NBT tag",
      Self::Time => "a time duration (10 (ticks), 0.5d (days), 3s (seconds))",
      Self::Modid => "a mod id",
      Self::Enum => "an enum added by Forge",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct NoneSender {}

  impl CommandSender for NoneSender {
    fn block_pos(&self) -> Option<Pos> {
      None
    }
  }

  #[test]
  fn parse_types() -> Result<()> {
    assert_eq!(Parser::Bool.parse(&mut Tokenizer::new("true"), &NoneSender {})?, Arg::Bool(true));
    assert_eq!(Parser::Bool.parse(&mut Tokenizer::new("false"), &NoneSender {})?, Arg::Bool(false));
    assert_eq!(
      Parser::Bool.parse(&mut Tokenizer::new("invalid"), &NoneSender {}).unwrap_err().kind(),
      &ErrorKind::Expected("true or false".into())
    );

    // assert_eq!(Parser::Double { min: None, max: None }.parse("5.3")?,
    // (Arg::Double(5.3), 3)); assert_eq!(Parser::Double { min: None, max: None
    // }.parse("3.0000")?, (Arg::Double(3.0), 6)); assert_eq!(
    //   Parser::Double { min: Some(1.0), max: None }.parse("-5"),
    //   Err(ParseError::Range(-5.0, Some(1.0), None))
    // );
    //
    // assert_eq!(Parser::Float { min: None, max: None }.parse("5.3")?,
    // (Arg::Float(5.3), 3)); assert_eq!(Parser::Float { min: None, max: None
    // }.parse("3.0000")?, (Arg::Float(3.0), 6)); assert_eq!(
    //   Parser::Float { min: Some(1.0), max: None }.parse("-5"),
    //   Err(ParseError::Range(-5.0, Some(1.0), None))
    // );
    //
    // assert_eq!(Parser::Int { min: None, max: None }.parse("5")?, (Arg::Int(5),
    // 1)); assert_eq!(Parser::Int { min: None, max: None }.parse("03")?,
    // (Arg::Int(3), 2)); assert_eq!(
    //   Parser::Int { min: None, max: None }.parse("3.2"),
    //   Err(ParseError::InvalidText("3.2".into(), "an int".into()))
    // );
    // assert_eq!(
    //   Parser::Int { min: Some(1), max: None }.parse("-5"),
    //   Err(ParseError::Range(-5.0, Some(1.0), None))
    // );
    //
    // assert_eq!(
    //   Parser::String(StringType::Word).parse("big gaming")?,
    //   (Arg::String("big".into()), 3)
    // );
    // assert_eq!(Parser::String(StringType::Word).parse("word")?,
    // (Arg::String("word".into()), 4)); assert_eq!(
    //   Parser::String(StringType::Word).parse(""),
    //   Err(ParseError::EOF(Parser::String(StringType::Word))),
    // );
    // assert_eq!(
    //   Parser::String(StringType::Quotable).parse("big gaming")?,
    //   (Arg::String("big".into()), 3)
    // );
    // assert_eq!(
    //   Parser::String(StringType::Quotable).parse("\"big gaming\" things")?,
    //   (Arg::String("big gaming".into()), 12) // 10 + 2 because of the quotes
    // );
    // assert_eq!(
    //   Parser::String(StringType::Quotable).parse(r#""big gam\"ing" things"#)?,
    //   (Arg::String(r#"big gam"ing"#.into()), 14) // 11 + 2 + 1 because of the
    // quotes and \ );
    // assert_eq!(
    //   Parser::String(StringType::Quotable).parse(r#""big gam\\\"ing" things"#)?,
    //   (Arg::String(r#"big gam\"ing"#.into()), 16)
    // );
    // assert_eq!(
    //   Parser::String(StringType::Quotable).parse(r#""big gam\\"ing" things"#)?,
    //   (Arg::String(r#"big gam\"#.into()), 11)
    // );
    // assert_eq!(
    //   Parser::String(StringType::Greedy).parse(r#""big gam\\"ing" things"#)?,
    //   (Arg::String(r#""big gam\\"ing" things"#.into()), 22)
    // );
    //
    // assert_eq!(
    //   Parser::BlockPos.parse("10 12"),
    //   Err(ParseError::InvalidText("10 12".into(), "a block position".into())),
    // );
    // assert_eq!(Parser::BlockPos.parse("10 12 15")?, (Arg::BlockPos(Pos::new(10,
    // 12, 15)), 8)); assert_eq!(Parser::BlockPos.parse("10 12 15 20")?,
    // (Arg::BlockPos(Pos::new(10, 12, 15)), 8));

    // Parser::Entity { single, players } => (),
    // Parser::ScoreHolder { multiple } => (),
    // Parser::GameProfile => (),
    // Parser::BlockPos => (),
    // Parser::ColumnPos => (),
    // Parser::Vec3 => (),
    // Parser::Vec2 => (),
    // Parser::BlockState => (),
    // Parser::BlockPredicate => (),
    // Parser::ItemStack => (),
    // Parser::ItemPredicate => (),
    // Parser::Color => (),
    // Parser::Component => (),
    // Parser::Message => (),
    // Parser::Nbt => (),
    // Parser::NbtPath => (),
    // Parser::Objective => (),
    // Parser::ObjectiveCriteria => (),
    // Parser::Operation => (),
    // Parser::Particle => (),
    // Parser::Rotation => (),
    // Parser::Angle => (),
    // Parser::ScoreboardSlot => (),
    // Parser::Swizzle => (),
    // Parser::Team => (),
    // Parser::ItemSlot => (),
    // Parser::ResourceLocation => (),
    // Parser::MobEffect => (),
    // Parser::Function => (),
    // Parser::EntityAnchor => (),
    // Parser::Range { decimals: bool } => (),
    // Parser::IntRange => (),
    // Parser::FloatRange => (),
    // Parser::ItemEnchantment => (),
    // Parser::EntitySummon => (),
    // Parser::Dimension => (),
    // Parser::Uuid => (),
    // Parser::NbtTag => (),
    // Parser::NbtCompoundTag => (),
    // Parser::Time => (),
    // Parser::Modid => (),
    // Parser::Enum => (),
    Ok(())
  }
}
