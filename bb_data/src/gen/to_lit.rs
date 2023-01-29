//! Converts things to literals.
use super::CodeGen;

pub trait ToLit {
  fn to_lit(&self, gen: &mut CodeGen);
}

impl ToLit for u8 {
  fn to_lit(&self, gen: &mut CodeGen) { gen.write(&self.to_string()); }
}
impl ToLit for u32 {
  fn to_lit(&self, gen: &mut CodeGen) { gen.write(&self.to_string()); }
}
impl ToLit for i32 {
  fn to_lit(&self, gen: &mut CodeGen) { gen.write(&self.to_string()); }
}
impl ToLit for f32 {
  fn to_lit(&self, gen: &mut CodeGen) {
    if self.fract() == 0.0 {
      gen.write(&self.to_string());
      gen.write(".0");
    } else {
      gen.write(&self.to_string());
    }
  }
}
impl ToLit for String {
  fn to_lit(&self, gen: &mut CodeGen) {
    gen.write("\"");
    gen.write(self);
    gen.write("\"");
  }
}
impl<T> ToLit for Vec<T>
where
  T: ToLit,
{
  fn to_lit(&self, gen: &mut CodeGen) {
    if self.is_empty() {
      gen.write("&[]");
      return;
    }
    gen.write_line("&[");
    gen.add_indent();
    for (_i, p) in self.iter().enumerate() {
      p.to_lit(gen);
      gen.write_line(",");
    }
    gen.remove_indent();
    gen.write("]");
  }
}
