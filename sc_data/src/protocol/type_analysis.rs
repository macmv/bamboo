use super::{Instr, Packet, RType, Type};

impl Packet {
  pub fn find_reader_types(&mut self) {
    self.find_instr(&self.reader.clone());
  }

  fn find_instr(&mut self, instr: &[Instr]) {}
}
