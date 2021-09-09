mod err;
mod token;

pub use err::{ErrorKind, ParseError, Result};
pub use token::{Span, Tokenizer};

use super::{Arg, Parser};

impl Parser {
  pub fn parse(&self, tokens: &mut Tokenizer) -> Result<Arg> {
    match self {
      Self::Bool => {
        let w = tokens.read_word()?;
        if w == "true" {
          Ok(Arg::Bool(true))
        } else if w == "false" {
          Ok(Arg::Bool(false))
        } else {
          Err(w.expected("true or false"))
        }
      }
      _ => unimplemented!(),
      // Self::Double { min, max } => {
      //   parse_num(text, *min, *max, "a double").map(|(num, len)| (Arg::Double(num), len))
      // }
      // Self::Float { min, max } => {
      //   parse_num(text, *min, *max, "a float").map(|(num, len)| (Arg::Float(num), len))
      // }
      // Self::Int { min, max } => {
      //   parse_num(text, *min, *max, "an int").map(|(num, len)| (Arg::Int(num), len))
      // }
      // Self::String(ty) => match ty {
      //   StringType::Word => {
      //     if text.is_empty() {
      //       return Err(ParseError::EOF);
      //     }
      //     let word = parse_word(text);
      //     let len = word.len();
      //     Ok((Arg::String(word), len))
      //   }
      //   StringType::Quotable => {
      //     if text.is_empty() {
      //       return Err(ParseError::EOF);
      //     }
      //     let mut iter = text.chars();
      //     if iter.next().unwrap() == '"' {
      //       let mut escaping = false;
      //       let mut index = 1;
      //       let mut out = String::new();
      //       for c in iter {
      //         if escaping {
      //           if c == '"' || c == '\\' {
      //             out.push(c);
      //           } else {
      //             return Err(ParseError::InvalidText(
      //               text.into(),
      //               "a valid escape character".into(),
      //             ));
      //           }
      //           escaping = false;
      //         } else {
      //           if c == '"' {
      //             break;
      //           } else if c == '\\' {
      //             escaping = true;
      //           } else {
      //             out.push(c);
      //           }
      //         }
      //         index += 1;
      //       }
      //       // Add 1 so that the ending quote is removed
      //       Ok((Arg::String(out), index + 1))
      //     } else {
      //       let mut index = 1;
      //       for c in iter {
      //         if c == ' ' {
      //           break;
      //         }
      //         index += 1;
      //       }
      //       Ok((Arg::String(text[0..index].into()), index))
      //     }
      //   }
      //   StringType::Greedy => Ok((Arg::String(text.into()), text.len())),
      // },
      // Self::Entity { single: _, players: _ } => Ok((Arg::Int(5), 1)),
      // Self::ScoreHolder { multiple: _ } => Ok((Arg::Int(5), 1)),
      // Self::GameProfile => Ok((Arg::Int(5), 1)),
      // Self::BlockPos => {
      //   let sections: Vec<&str> = text.split(' ').collect();
      //   if sections.len() < 3 {
      //     return Err(ParseError::InvalidText(text.into(), "a block position".into()));
      //   }
      //   let x = sections[0]
      //     .parse()
      //     .map_err(|_| ParseError::InvalidText(text.into(), "a valid block position".into()))?;
      //   let y = sections[1]
      //     .parse()
      //     .map_err(|_| ParseError::InvalidText(text.into(), "a valid block position".into()))?;
      //   let z = sections[2]
      //     .parse()
      //     .map_err(|_| ParseError::InvalidText(text.into(), "a valid block position".into()))?;
      //   Ok((
      //     Arg::BlockPos(Pos::new(x, y, z)),
      //     sections[0].len() + sections[1].len() + sections[2].len() + 2,
      //   ))
      // }
      // Self::ColumnPos => Ok((Arg::Int(5), 1)),
      // Self::Vec3 => Ok((Arg::Int(5), 1)),
      // Self::Vec2 => Ok((Arg::Int(5), 1)),
      // Self::BlockState => {
      //   let word = parse_word(text);
      //   Ok((
      //     Arg::BlockState(
      //       block::Kind::from_str(&word)
      //         .map_err(|_| ParseError::InvalidText(text.into(), "a valid block name".into()))?,
      //       HashMap::new(),
      //       None,
      //     ),
      //     word.len(),
      //   ))
      // }
      // Self::BlockPredicate => Ok((Arg::Int(5), 1)),
      // Self::ItemStack => Ok((Arg::Int(5), 1)),
      // Self::ItemPredicate => Ok((Arg::Int(5), 1)),
      // Self::Color => Ok((Arg::Int(5), 1)),
      // Self::Component => Ok((Arg::Int(5), 1)),
      // Self::Message => Ok((Arg::Int(5), 1)),
      // Self::Nbt => Ok((Arg::Int(5), 1)),
      // Self::NbtPath => Ok((Arg::Int(5), 1)),
      // Self::Objective => Ok((Arg::Int(5), 1)),
      // Self::ObjectiveCriteria => Ok((Arg::Int(5), 1)),
      // Self::Operation => Ok((Arg::Int(5), 1)),
      // Self::Particle => Ok((Arg::Int(5), 1)),
      // Self::Rotation => Ok((Arg::Int(5), 1)),
      // Self::Angle => Ok((Arg::Int(5), 1)),
      // Self::ScoreboardSlot => Ok((Arg::Int(5), 1)),
      // Self::Swizzle => Ok((Arg::Int(5), 1)),
      // Self::Team => Ok((Arg::Int(5), 1)),
      // Self::ItemSlot => Ok((Arg::Int(5), 1)),
      // Self::ResourceLocation => Ok((Arg::Int(5), 1)),
      // Self::MobEffect => Ok((Arg::Int(5), 1)),
      // Self::Function => Ok((Arg::Int(5), 1)),
      // Self::EntityAnchor => Ok((Arg::Int(5), 1)),
      // Self::Range { decimals: _bool } => Ok((Arg::Int(5), 1)),
      // Self::IntRange => Ok((Arg::Int(5), 1)),
      // Self::FloatRange => Ok((Arg::Int(5), 1)),
      // Self::ItemEnchantment => Ok((Arg::Int(5), 1)),
      // Self::EntitySummon => Ok((Arg::Int(5), 1)),
      // Self::Dimension => Ok((Arg::Int(5), 1)),
      // Self::Uuid => Ok((Arg::Int(5), 1)),
      // Self::NbtTag => Ok((Arg::Int(5), 1)),
      // Self::NbtCompoundTag => Ok((Arg::Int(5), 1)),
      // Self::Time => Ok((Arg::Int(5), 1)),
      // Self::Modid => Ok((Arg::Int(5), 1)),
      // Self::Enum => Ok((Arg::Int(5), 1)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_types() -> Result<()> {
    assert_eq!(Parser::Bool.parse(&mut Tokenizer::new("true"))?, Arg::Bool(true));
    assert_eq!(Parser::Bool.parse(&mut Tokenizer::new("false"))?, Arg::Bool(false));

    let mut tok = Tokenizer::new("false true");
    assert_eq!(Parser::Bool.parse(&mut tok).unwrap(), Arg::Bool(false));
    assert_eq!(Parser::Bool.parse(&mut tok).unwrap(), Arg::Bool(true));
    assert_eq!(Parser::Bool.parse(&mut tok).unwrap_err().kind(), &ErrorKind::EOF);
    assert_eq!(Parser::Bool.parse(&mut tok).unwrap_err().kind(), &ErrorKind::EOF);

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
