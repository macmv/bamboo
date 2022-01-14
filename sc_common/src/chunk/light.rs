pub struct LightChunk {
  sections: Vec<Option<LightSection>>,
}

pub struct LightSection {
  /// 2048 bytes, each representing 2 blocks.
  data: Vec<u8>,
}
