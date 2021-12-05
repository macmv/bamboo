use super::{
  convert, simplify, Cond, Expr, Field, Instr, Lit, Op, Packet, PacketDef, RType, Type, Value,
  VarKind,
};

impl Packet {
  pub fn extend_from(&mut self, sup: &Packet) {
    let old = self.fields.clone();
    self.fields = sup.fields.clone();
    self.fields.extend(old);

    let mut new = Vec::with_capacity(self.reader.block.len() + sup.reader.block.len());
    for i in &self.reader.block {
      match i {
        Instr::Super => new.extend(sup.reader.block.clone()),
        _ => new.push(i.clone()),
      }
    }
    self.reader.block = new;
  }
}
