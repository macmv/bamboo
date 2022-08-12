use std::{error::Error, fmt, str::FromStr};

/// Any data specific to a block kind. This includes all function handlers for
/// when a block gets placed/broken, and any custom functionality a block might
/// have.
#[derive(Debug)]
pub struct Data {
  name: &'static str,
  id:   u32,
}

impl Data {
  /// Returns the particle's ID. This is the latest protocol ID.
  pub fn id(&self) -> u32 { self.id }
  /// Returns the name of this particle. This is something like `dust`. These
  /// don't have namespaces, because there aren't any namespaces for these on
  /// vanilla.
  ///
  /// TODO: Add namespaces.
  pub fn name(&self) -> &'static str { self.name }
}

#[derive(Debug)]
pub struct InvalidEnchantment(String);

impl fmt::Display for InvalidEnchantment {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid enchantment name: {}", self.0)
  }
}

impl Error for InvalidEnchantment {}

// Creates the type enum, and the generate_data function
include!(concat!(env!("OUT_DIR"), "/enchantment/ty.rs"));
