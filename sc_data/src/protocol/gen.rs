use super::{Cond, Expr, Instr, Lit, Op, Packet, PacketDef, Value, Var};
use crate::{gen::CodeGen, Version};
use convert_case::{Case, Casing};
use std::{collections::HashMap, fs, fs::File, io, io::Write, path::Path};

pub fn generate(def: Vec<(Version, PacketDef)>, dir: &Path) -> io::Result<()> {
  let mut all_cb_packets = PacketCollection::new();
  let mut all_sb_packets = PacketCollection::new();

  for (ver, def) in def {
    for p in def.clientbound {
      all_cb_packets.add(ver, p);
    }
    for p in def.serverbound {
      all_sb_packets.add(ver, p);
    }
  }

  fs::create_dir_all(dir)?;
  File::create(dir.join("cb.rs"))?.write_all(all_cb_packets.gen().as_bytes())?;
  File::create(dir.join("sb.rs"))?.write_all(all_sb_packets.gen().as_bytes())?;

  Ok(())
}
struct PacketCollection {
  packets: HashMap<String, Vec<(Version, Packet)>>,
}

impl PacketCollection {
  pub fn new() -> Self {
    PacketCollection { packets: HashMap::new() }
  }
  pub fn add(&mut self, ver: Version, mut p: Packet) {
    sanitize(&mut p);
    let list = self.packets.entry(p.name.clone()).or_insert_with(|| vec![]);
    if let Some((_, last)) = list.last() {
      if *last == p {
        return;
      }
    }
    list.push((ver, p));
  }
  pub fn gen(self) -> String {
    let mut gen = CodeGen::new();

    let mut packets: Vec<_> = self.packets.into_iter().collect();
    packets.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    let packets: Vec<Vec<(_, _)>> = packets.into_iter().map(|(_, v)| v).collect();

    gen.write("pub enum Packet ");
    gen.write_block(|gen| {
      for versions in &packets {
        for (ver, p) in versions {
          write_packet(gen, &format!("{}V{}", p.name, ver.maj), p);
        }
      }
    });

    gen.write_impl("Packet", |gen| {
      gen.write("pub fn from_tcp(p: tcp::Packet, ver: ProtocolVersion) -> Self ");
      gen.write_block(|gen| {
        gen.write_match("to_sug_id(p.id(), ver)", |gen| {
          for (id, versions) in packets.iter().enumerate() {
            gen.write(&id.to_string());
            gen.write(" => ");
            gen.write_block(|gen| {
              let (ver, first) = versions.first().unwrap();
              gen.write_comment(&first.name);
              if ver.maj != 8 {
                gen.write("if ver < ");
                gen.write(&ver.maj.to_string());
                gen.write(" ");
                gen.write_block(|gen| {
                  gen.write(r#"panic!("version {} is below the minimum version for packet "#);
                  gen.write(&first.name);
                  gen.write_line(r#"", ver);"#);
                });
              }
              if versions.len() == 1 {
                write_from_tcp(gen, first, *ver);
              } else {
                for (i, (ver, p)) in versions.iter().enumerate() {
                  if let Some(next_ver) = versions.get(i + 1) {
                    gen.write("if ver < ");
                    gen.write(&next_ver.0.maj.to_string());
                    gen.write_line(" {");
                    gen.add_indent();
                    write_from_tcp(gen, p, *ver);
                    gen.remove_indent();
                    gen.write("} else ");
                  } else {
                    gen.write_block(|gen| {
                      write_from_tcp(gen, p, *ver);
                    });
                  }
                }
              }
            });
          }
        });
      });
    });

    gen.into_output()
  }
}

fn sanitize(p: &mut Packet) {
  for f in &mut p.fields {
    simplify_name(&mut f.name);
  }
}
fn simplify_name(name: &mut String) {
  if name == "type" {
    *name = "ty".into();
  } else {
    *name = name.to_case(Case::Snake);
  }
}

fn write_packet(gen: &mut CodeGen, name: &str, p: &Packet) {
  gen.write(name);
  gen.write_line(" {");
  gen.add_indent();
  for f in &p.fields {
    gen.write(&f.name);
    gen.write(": ");
    gen.write(&f.ty.to_rust());
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("},");
}

fn write_from_tcp(gen: &mut CodeGen, p: &Packet, ver: Version) {
  for i in &p.reader {
    write_instr(gen, i);
  }
  gen.write(&p.name);
  gen.write("V");
  gen.write(&ver.maj.to_string());
  gen.write_line(" {");
  gen.add_indent();
  for f in &p.fields {
    gen.write(&f.name);
    gen.write(": f_");
    gen.write(&f.name);
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("}");
}

fn write_instr(gen: &mut CodeGen, i: &Instr) {
  match i {
    Instr::Super => {}
    Instr::Set(name, val) => {
      let mut name = name.clone();
      gen.write("let f_");
      simplify_name(&mut name);
      gen.write(&name);
      gen.write(" = ");
      write_expr(gen, val);
      gen.write_line(";");
    }
    Instr::SetArr(arr, idx, val) => {
      write_val(gen, arr);
      gen.write("[");
      write_val(gen, idx);
      gen.write("] = ");
      write_expr(gen, val);
      gen.write_line(";");
    }
    Instr::Let(var, val) => {
      gen.write("let v_");
      gen.write(&var.to_string());
      gen.write(" = ");
      write_expr(gen, val);
      gen.write_line(";");
    }
    Instr::Call(v, name, args) => {
      if v.initial != Value::Null {
        write_expr(gen, v);
        gen.write(".");
      }
      gen.write(name);
      gen.write("(");
      for (i, a) in args.iter().enumerate() {
        write_expr(gen, a);
        if i != args.len() - 1 {
          gen.write(", ");
        }
      }
      gen.write_line(");");
    }
    Instr::If(cond, true_block, false_block) => {
      gen.write("if ");
      write_cond(gen, cond);
      gen.write_line(" {");
      gen.add_indent();
      for i in true_block {
        write_instr(gen, i);
      }
      gen.remove_indent();
      if !false_block.is_empty() {
        gen.write_line("} else {");
        gen.add_indent();
        for i in false_block {
          write_instr(gen, i);
        }
        gen.remove_indent();
      }
      gen.write_line("}");
    }
    Instr::For(v, range, block) => {
      gen.write("for ");
      if let Var::Local(v) = v {
        gen.write("v_");
        gen.write(&v.to_string());
      } else {
        panic!("cannot iterate with self or buf as the value");
      }
      gen.write(" in ");
      write_expr(gen, &range.min);
      gen.write("..");
      write_expr(gen, &range.max);
      gen.write_line(" {");
      gen.add_indent();
      for i in block {
        write_instr(gen, i);
      }
      gen.remove_indent();
      gen.write_line("}");
    }
    Instr::Switch(v, items) => {}
    Instr::CheckStrLen(_, _) => {}
  }
}

fn write_expr(gen: &mut CodeGen, e: &Expr) {
  let mut g = CodeGen::new();
  write_val(&mut g, &e.initial);
  let mut val = g.into_output();
  for (i, op) in e.ops.iter().enumerate() {
    let needs_paren = i
      .checked_sub(1)
      .map(|i| {
        let prev = &e.ops[i];
        prev.precedence() > op.precedence()
      })
      .unwrap_or(false);
    let mut g = CodeGen::new();
    if needs_paren {
      g.write("(");
    }
    match op {
      Op::BitAnd(rhs) => {
        g.write(&val);
        g.write(" & ");
        write_expr(&mut g, rhs);
      }
      Op::Shr(rhs) => {
        g.write(&val);
        g.write(" >> ");
        write_expr(&mut g, rhs);
      }
      Op::UShr(rhs) => {
        g.write(&val);
        g.write(" >> ");
        write_expr(&mut g, rhs);
      }
      Op::Shl(rhs) => {
        g.write(&val);
        g.write(" << ");
        write_expr(&mut g, rhs);
      }

      Op::Add(rhs) => {
        g.write(&val);
        g.write(" + ");
        write_expr(&mut g, rhs);
      }
      Op::Div(rhs) => {
        g.write(&val);
        g.write(" / ");
        write_expr(&mut g, rhs);
      }

      Op::Len => {
        g.write(&val);
        g.write(".len()");
      }
      Op::Idx(rhs) => {
        g.write(&val);
        g.write("[");
        write_expr(&mut g, rhs);
        g.write("]");
      }
      Op::CollectionIdx(idx) => {
        g.write(&val);
        g.write(".");
        g.write(&idx.to_string());
      }

      Op::If(cond, new) => {
        g.write("if ");
        write_cond(&mut g, cond);
        g.write(" { ");
        write_expr(&mut g, new);
        g.write(" } else { ");
        g.write(&val);
        g.write(" }");
      }
    }
    if needs_paren {
      g.write(")");
    }
    val = g.into_output();
  }
  gen.write(&val);
}

fn write_val(gen: &mut CodeGen, val: &Value) {
  match val {
    Value::Null => gen.write("None"),
    Value::Lit(lit) => match lit {
      Lit::Int(v) => gen.write(&v.to_string()),
      Lit::Float(v) => gen.write(&v.to_string()),
      Lit::String(v) => gen.write(&v),
    },
    Value::Var(v) => match v {
      Var::This => gen.write("self"),
      Var::Buf => gen.write("p"),
      Var::Local(v) => {
        gen.write("v_");
        gen.write(&v.to_string())
      }
    },
    Value::Static(class, name) => {
      for s in class.split('/').last().unwrap().split('$') {
        gen.write(s);
      }
      gen.write(".");
      gen.write(name);
    }
    Value::Field(name) => {
      gen.write("f_");
      gen.write(name);
    }
    Value::Array(len) => {
      gen.write("Vec::with_capacity(");
      write_expr(gen, len);
      gen.write(")");
    }
    Value::Call(val, name, args) => {
      if let Some(e) = val {
        write_expr(gen, e);
        gen.write(".");
      }
      gen.write(name);
      gen.write("(");
      for (i, a) in args.iter().enumerate() {
        write_expr(gen, a);
        if i != args.len() - 1 {
          gen.write(", ");
        }
      }
      gen.write(")");
    }
    Value::Collection(name, args) => {
      gen.write(name.split('/').last().unwrap().split('$').last().unwrap());
      gen.write("::new(");
      for (i, a) in args.iter().enumerate() {
        write_expr(gen, a);
        if i != args.len() - 1 {
          gen.write(", ");
        }
      }
      gen.write(")");
    }
  }
}

fn write_cond(gen: &mut CodeGen, cond: &Cond) {
  macro_rules! cond {
    ($gen:expr, $lhs:ident $comp:tt $rhs:ident) => {{
      write_expr(gen, $lhs);
      $gen.write(concat!(" ", stringify!($comp), " "));
      write_expr(gen, $rhs);
    }};
  }
  match cond {
    Cond::Eq(lhs, rhs) => cond!(gen, lhs == rhs),
    Cond::Neq(lhs, rhs) => cond!(gen, lhs != rhs),
    Cond::Less(lhs, rhs) => cond!(gen, lhs < rhs),
    Cond::Greater(lhs, rhs) => cond!(gen, lhs > rhs),
    Cond::Lte(lhs, rhs) => cond!(gen, lhs <= rhs),
    Cond::Gte(lhs, rhs) => cond!(gen, lhs >= rhs),

    Cond::Or(lhs, rhs) => {
      gen.write("(");
      write_cond(gen, lhs);
      gen.write(") || (");
      write_cond(gen, rhs);
      gen.write(")");
    }
  }
}
