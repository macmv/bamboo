use super::{add_from, wrap};
use crate::block;
use std::str::FromStr;
use sugarlang::{define_ty, parse::token::Span, runtime::RuntimeError};

wrap!(block::Kind, SlBlockKind);

/// A block kind. This is how you get/set blocks in the world.
#[define_ty(path = "sugarcane::block::BlockKind")]
impl SlBlockKind {
  pub fn from_s(name: &str) -> Result<SlBlockKind, RuntimeError> {
    Ok(SlBlockKind {
      inner: block::Kind::from_str(name).map_err(|_| {
        RuntimeError::custom(format!("invalid block name '{}'", name), Span::default())
      })?,
    })
  }
  pub fn to_s(&self) -> String {
    format!("{}", self.inner.to_str())
  }
}
