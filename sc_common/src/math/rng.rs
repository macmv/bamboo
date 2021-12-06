use rand_core::{impls, Error, RngCore};

#[derive(Debug)]
pub struct WyhashRng(u64);

impl WyhashRng {
  pub fn new(seed: u64) -> Self { WyhashRng(seed) }
}

impl RngCore for WyhashRng {
  fn next_u64(&mut self) -> u64 {
    self.0 = self.0.wrapping_add(0x60bee2bee120fc15);
    let mut tmp = self.0 as u128 * 0xa3b195354a39b70d;
    let m1 = ((tmp >> 64) ^ tmp) as u64;
    tmp = m1 as u128 * 0x1b03738712fad5c9;
    ((tmp >> 64) ^ tmp) as u64
  }
  fn next_u32(&mut self) -> u32 { self.next_u64() as u32 }
  fn fill_bytes(&mut self, dest: &mut [u8]) { impls::fill_bytes_via_next(self, dest) }
  fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
    self.fill_bytes(dest);
    Ok(())
  }
}
