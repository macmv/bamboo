use super::ParseError;
use std::str::FromStr;

pub fn parse_num<T>(
  text: &str,
  min: Option<T>,
  max: Option<T>,
  expected: &str,
) -> Result<(T, usize), ParseError>
where
  T: PartialOrd + FromStr + Into<f64> + Copy,
{
  let section = &text[..text.find(' ').unwrap_or(text.len())];
  match section.parse::<T>() {
    Ok(v) => {
      let mut invalid = false;
      if let Some(min) = min {
        if v < min {
          invalid = true;
        }
      }
      if let Some(max) = max {
        if v > max {
          invalid = true;
        }
      }
      if invalid {
        Err(ParseError::Range(v.into(), min.map(|v| v.into()), max.map(|v| v.into())))
      } else {
        Ok((v, section.len()))
      }
    }
    Err(_) => Err(ParseError::InvalidText(text.into(), expected.into())),
  }
}

pub fn parse_word(text: &str) -> String {
  text[..text.find(' ').unwrap_or(text.len())].into()
}
