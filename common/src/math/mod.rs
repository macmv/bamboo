use crate::proto;

#[derive(Debug, Clone, Copy)]
pub struct UUID(u128);

impl UUID {
  pub fn from_u128(v: u128) -> Self {
    UUID(v)
  }
  pub fn as_proto(&self) -> proto::Uuid {
    proto::Uuid { be_data: self.as_be_bytes().to_vec() }
  }
  /// Returns the uuid represented as a hex string, with no dashes or other
  /// characters.
  pub fn as_str(&self) -> String {
    format!("{}", self.0)
  }
  /// Returns the uuid represented as a string with dashes. This is used
  /// sometimes when refering to player in json, and is a useful function to
  /// have.
  pub fn as_dashed_str(&self) -> String {
    format!(
      "{:x}-{:x}-{:x}-{:x}-{:x}",
      //         11111111222233334444555555555555
      (self.0 & 0xffffffff000000000000000000000000) >> 24 * 4, // 4 bits per digit
      (self.0 & 0x00000000ffff00000000000000000000) >> 20 * 4,
      (self.0 & 0x000000000000ffff0000000000000000) >> 16 * 4,
      (self.0 & 0x0000000000000000ffff000000000000) >> 12 * 4,
      (self.0 & 0x00000000000000000000ffffffffffff),
    )
  }
  /// Returns the underlying `u128`. For packets, you probably want
  /// [`as_be_bytes`](Self::as_be_bytes). For json, you probably want
  /// [`as_str`](Self::as_str) or [`as_dashed_str`](Self::as_dashed_str).
  pub fn as_u128(&self) -> u128 {
    self.0
  }
  /// Returns the little-endian representation of the underlying `u128`. This is
  /// the byte order that the Minecraft Bedrock Edition uses in its packet
  /// protocol.
  pub fn as_le_bytes(&self) -> [u8; 16] {
    self.0.to_le_bytes()
  }
  /// Returns the big-endian representation of the underlying `u128`. This is
  /// the byte order that the Minecraft Java Edition uses in its packet
  /// protocol.
  pub fn as_be_bytes(&self) -> [u8; 16] {
    self.0.to_be_bytes()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn uuid_dashed_str() {
    let uuid = UUID::from_u128(0x11111111222233334444555555555555);
    assert_eq!(uuid.as_dashed_str(), "11111111-2222-3333-4444-555555555555");
  }
}
