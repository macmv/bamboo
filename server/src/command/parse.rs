use super::{Arg, Parser};
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum ParseError {
  /// Used when a literal does not match
  InvalidLiteral(String),
  /// Used when no children of the node matched
  NoChildren(Vec<ParseError>),
  /// Used when there are trailing characters after the command
  Trailing(String),
  /// Used whenever a field does not match the given text
  InvalidText(String, String),
  /// Used when a value is out of range
  Range(f64, Option<f64>, Option<f64>),
}

impl fmt::Display for ParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidLiteral(v) => write!(f, "invalid literal: {}", v),
      Self::NoChildren(errors) => {
        if errors.is_empty() {
          // No errors means print another error about no errors
          write!(f, "no errors in no children error (should never happen)")
        } else if errors.len() == 1 {
          // A single error should just be printed as that error
          write!(f, "{}", errors[0])
        } else {
          // Write all of the children in a row
          writeln!(f, "no children matched: [")?;
          for e in errors {
            write!(f, "  {}", e)?;
          }
          write!(f, "]")
        }
      }
      Self::Trailing(v) => write!(f, "trailing characters: {}", v),
      Self::InvalidText(text, expected) => {
        write!(f, "invalid text: {}. expected {}", text, expected)
      }
      Self::Range(v, min, max) => {
        if let Some(min) = min {
          if let Some(max) = max {
            write!(f, "{} is out of range {}..{}", v, min, max)
          } else {
            write!(f, "{} is less than min {}", v, min)
          }
        } else {
          if let Some(max) = max {
            write!(f, "{} is greater than max {}", v, max)
          } else {
            write!(f, "{} is out of range none (should never happen)", v)
          }
        }
      }
    }
  }
}

impl Error for ParseError {}

impl Parser {
  pub fn parse(&self, text: &str) -> Result<(Arg, usize), ParseError> {
    match self {
      Self::Bool => {
        if text.starts_with("true") {
          Ok((Arg::Bool(true), 5))
        } else if text.starts_with("false") {
          Ok((Arg::Bool(false), 6))
        } else {
          Err(ParseError::InvalidText(text.into(), "true or false".into()))
        }
      }
      Self::Double { min, max } => {
        let section = &text[..text.find(' ').unwrap_or(0)];
        match section.parse() {
          Ok(v) => {
            let mut invalid = false;
            if let Some(min) = min {
              if v < *min {
                invalid = true;
              }
            }
            if let Some(max) = max {
              if v > *max {
                invalid = true;
              }
            }
            if invalid {
              Err(ParseError::Range(v, *min, *max))
            } else {
              Ok((Arg::Double(v), section.len()))
            }
          }
          Err(e) => Err(ParseError::InvalidText(text.into(), "a double".into())),
        }
      }
      Self::Float { min, max } => (),
      Self::Int { min, max } => (),
      Self::String(StringType) => (),
      Self::Entity { single, players } => (),
      Self::ScoreHolder { multiple } => (),
      Self::GameProfile => (),
      Self::BlockPos => (),
      Self::ColumnPos => (),
      Self::Vec3 => (),
      Self::Vec2 => (),
      Self::BlockState => (),
      Self::BlockPredicate => (),
      Self::ItemStack => (),
      Self::ItemPredicate => (),
      Self::Color => (),
      Self::Component => (),
      Self::Message => (),
      Self::Nbt => (),
      Self::NbtPath => (),
      Self::Objective => (),
      Self::ObjectiveCriteria => (),
      Self::Operation => (),
      Self::Particle => (),
      Self::Rotation => (),
      Self::Angle => (),
      Self::ScoreboardSlot => (),
      Self::Swizzle => (),
      Self::Team => (),
      Self::ItemSlot => (),
      Self::ResourceLocation => (),
      Self::MobEffect => (),
      Self::Function => (),
      Self::EntityAnchor => (),
      Self::Range { decimals: bool } => (),
      Self::IntRange => (),
      Self::FloatRange => (),
      Self::ItemEnchantment => (),
      Self::EntitySummon => (),
      Self::Dimension => (),
      Self::Uuid => (),
      Self::NbtTag => (),
      Self::NbtCompoundTag => (),
      Self::Time => (),
      Self::Modid => (),
      Self::Enum => (),
    }
  }
}
