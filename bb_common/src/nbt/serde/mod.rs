//! Serde bindings for NBT data.

mod de;
mod error;
mod ser;

pub use ser::{to_nbt, to_tag};
