use super::{convert, Cond, Expr, Instr, Lit, Op, Packet, PacketDef, Value, Var};
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
  simplify_instr(&mut p.reader);
  for f in &mut p.fields {
    simplify_name(&mut f.name);
    let (initialized, option) = check_option(&p.reader, &f.name);
    f.initialized = initialized;
    f.option = option;
  }
}
fn simplify_instr(instr: &mut [Instr]) {
  for i in instr {
    match i {
      Instr::Super => {}
      Instr::Set(name, v) => {
        simplify_name(name);
        simplify_expr(v);
      }
      Instr::SetArr(arr, idx, val) => {
        simplify_val(arr);
        simplify_val(idx);
        simplify_expr(val);
      }
      Instr::Let(_, val) => simplify_expr(val),
      Instr::Call(val, name, args) => {
        simplify_expr(val);
        simplify_name(name);
        if val.initial != Value::Null {
          let (new_name, arg) = convert::member_call(&name);
          *name = new_name.into();
          if let Some(a) = arg {
            *args = a;
          } else {
            args.iter_mut().for_each(|a| simplify_expr(a))
          }
        } else {
          let new_name = convert::static_call(&name);
          *name = new_name.into();
          args.iter_mut().for_each(|a| simplify_expr(a))
        }
      }
      Instr::If(cond, when_true, when_false) => {
        simplify_cond(cond);
        simplify_instr(when_true);
        simplify_instr(when_false);
      }
      Instr::For(_, range, block) => {
        simplify_expr(&mut range.min);
        simplify_expr(&mut range.max);
        simplify_instr(block);
      }
      Instr::Switch(val, items) => {
        simplify_expr(val);
        for (_, instr) in items {
          simplify_instr(instr);
        }
      }
      Instr::CheckStrLen(s, len) => {
        simplify_expr(s);
        simplify_val(len);
      }
    }
  }
}
fn simplify_cond(cond: &mut Cond) {
  match cond {
    Cond::Eq(lhs, rhs)
    | Cond::Neq(lhs, rhs)
    | Cond::Less(lhs, rhs)
    | Cond::Greater(lhs, rhs)
    | Cond::Lte(lhs, rhs)
    | Cond::Gte(lhs, rhs) => {
      simplify_expr(lhs);
      simplify_expr(rhs);
    }
    Cond::Or(lhs, rhs) => {
      simplify_cond(lhs);
      simplify_cond(rhs);
    }
  }
}
fn simplify_expr(expr: &mut Expr) {
  simplify_val(&mut expr.initial);
  expr.ops.iter_mut().for_each(|op| simplify_op(op))
}
fn simplify_val(val: &mut Value) {
  match val {
    Value::Null | Value::Lit(_) | Value::Var(_) => {}
    Value::Field(name) => simplify_name(name),
    Value::Static(..) => {}
    Value::Array(len) => simplify_expr(len),
    Value::Call(val, name, args) => {
      if let Some(v) = val {
        simplify_expr(v);
      }
      simplify_name(name);
      if val.is_some() {
        let (new_name, arg) = convert::member_call(&name);
        *name = new_name.into();
        if let Some(a) = arg {
          *args = a;
        } else {
          args.iter_mut().for_each(|a| simplify_expr(a))
        }
      } else {
        let new_name = convert::static_call(&name);
        *name = new_name.into();
        args.iter_mut().for_each(|a| simplify_expr(a))
      }
    }
    Value::Collection(_, args) => {
      args.iter_mut().for_each(|a| simplify_expr(a));
    }
  }
}
fn simplify_op(op: &mut Op) {
  match op {
    Op::BitAnd(rhs) => simplify_expr(rhs),
    Op::Shr(rhs) => simplify_expr(rhs),
    Op::UShr(rhs) => simplify_expr(rhs),
    Op::Shl(rhs) => simplify_expr(rhs),

    Op::Add(rhs) => simplify_expr(rhs),
    Op::Div(rhs) => simplify_expr(rhs),

    Op::Len => {}
    Op::Idx(idx) => simplify_expr(idx),
    Op::CollectionIdx(_) => {}

    Op::If(cond, val) => {
      simplify_cond(cond);
      simplify_expr(val)
    }
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
    if f.option {
      gen.write("Option<");
      gen.write(&f.ty.to_rust());
      gen.write(">");
    } else {
      gen.write(&f.ty.to_rust());
    }
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("},");
}

fn write_from_tcp(gen: &mut CodeGen, p: &Packet, ver: Version) {
  for f in &p.fields {
    gen.write("let");
    if !f.initialized {
      gen.write(" mut");
    }
    gen.write(" f_");
    gen.write(&f.name);
    if !f.initialized {
      gen.write(" = None");
    }
    gen.write_line(";");
  }
  let mut p2 = p.clone();
  for i in &p.reader {
    write_instr(gen, i, &mut p2);
  }
  let p = p2;
  gen.write("Packet::");
  gen.write(&p.name);
  gen.write("V");
  gen.write(&ver.maj.to_string());
  gen.write_line(" {");
  gen.add_indent();
  for f in &p.fields {
    gen.write(&f.name);
    gen.write(": f_");
    gen.write(&f.name);
    if let Some(read) = f.reader_type.as_ref() {
      let rs = f.ty.to_rust();
      if &rs != read {
        if f.option {
          gen.write(".map(|v| v");
          convert_ty(gen, read, &rs);
          gen.write(")");
        } else {
          convert_ty(gen, read, &rs);
        }
      }
    }
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("}");
}

fn convert_ty(gen: &mut CodeGen, from: &str, to: &str) {
  gen.write(match to {
    "bool" => " != 0",
    "f32" => " as f32",
    "f64" => " as f64",
    "u8" => match from {
      "i16" | "i32" | "i64" => ".try_into().unwrap()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "i16" => match from {
      "u8" => ".into()",
      "i32" | "i64" => ".try_into().unwrap()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "i32" => match from {
      "f32" => " as i32",
      "u8" | "i16" => ".into()",
      "i64" => ".try_into().unwrap()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "i64" => match from {
      "f32" => " as i64",
      "u8" | "i16" | "i32" => ".into()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    "HashMap<u8, u8>" | "HashMap<u8, i32>" | "HashSet<u8>" | "Vec<u8>" => return,
    "String" => match from {
      _ => return,
    },
    "Option<u8>" => match from {
      "i32" => ".unwrap_or(0).into()",
      _ => panic!("cannot convert `{}` into `{}`", from, to),
    },
    _ => panic!("cannot convert `{}` into `{}`", from, to),
  })
}

fn write_instr(gen: &mut CodeGen, instr: &Instr, p: &mut Packet) {
  match instr {
    Instr::Super => {
      gen.write_comment("call super here");
    }
    Instr::Set(name, val) => {
      gen.write("f_");
      gen.write(&name);
      gen.write(" = ");
      if let Some(field) = p.get_field_mut(&name) {
        match &val.initial {
          Value::Call(var, func, _)
            if var.as_ref().map(|v| v.initial == Value::Var(Var::Buf)).unwrap_or(false) =>
          {
            let ty = match func.as_str() {
              "read_boolean" => "bool",
              "read_varint" => "i32",
              "read_u8" => "u8",
              "read_i16" => "i16",
              "read_i32" => "i32",
              "read_optional" => "i32", // Literally used once in the entire 1.17 codebase.
              "read_i64" => "i64",
              "read_f32" => "f32",
              "read_f64" => "f64",
              "read_pos" => "Pos",
              "read_item" => "Stack",
              "read_uuid" => "UUID",
              "read_str" => "String",
              "read_nbt" => "NBT",
              "read_buf" => "Vec<u8>",
              "read_i32_arr" => "Vec<i32>",
              "read_varint_arr" => "Vec<i32>",
              "read_bit_set" => "BitSet",
              "read_block_hit" => "BlockHit",

              "read_map" => "u8",
              "read_list" => "u8",
              "read_collection" => "u8",
              _ => panic!("unknown reader function {}", func),
            };
            if let Some(ref reader) = field.reader_type {
              assert_eq!(reader, ty);
            } else {
              field.reader_type = Some(ty.into());
            }
          }
          // Conditionals as ops are always something like `if cond { 1 } else { 0 }`, which we can
          // convert with `v != 0`. So, in order to recognize that, we need to the reader type to be
          // a number.
          _ if matches!(&val.ops.last(), Some(Op::If(_, _))) => {
            if let Some(ref reader) = field.reader_type {
              assert_eq!(reader, "u8");
            } else {
              field.reader_type = Some("u8".into());
            }
          }
          _ => {}
        }
      }
      if p.get_field(&name).map(|f| f.option).unwrap_or(false) && val.initial != Value::Null {
        gen.write("Some(");
        write_expr(gen, val);
        gen.write(")");
      } else {
        write_expr(gen, val);
      }
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
      gen.write(&name);
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
        write_instr(gen, i, p);
      }
      gen.remove_indent();
      if !false_block.is_empty() {
        gen.write_line("} else {");
        gen.add_indent();
        for i in false_block {
          write_instr(gen, i, p);
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
        write_instr(gen, i, p);
      }
      gen.remove_indent();
      gen.write_line("}");
    }
    Instr::Switch(v, items) => {
      gen.write("match ");
      write_expr(gen, v);
      gen.write(" ");
      gen.write_block(|gen| {
        for (key, instr) in items {
          gen.write(&key.to_string());
          gen.write(" => ");
          gen.write_block(|gen| {
            for i in instr {
              write_instr(gen, i, p);
            }
          });
        }
      });
    }
    Instr::CheckStrLen(val, len) => {
      gen.write("assert!(");
      write_expr(gen, val);
      gen.write(".len() < ");
      write_val(gen, len);
      gen.write(", \"string is too long (len greater than `");
      write_val(gen, len);
      gen.write("`)\");");
    }
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
        g.write(".try_into().unwrap()]");
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
        g.write(&val);
        g.write(" } else { ");
        write_expr(&mut g, new);
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
      Lit::String(v) => {
        gen.write("\"");
        gen.write(&v);
        gen.write("\"");
      }
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
      gen.write(".try_into().unwrap())");
    }
    Value::Call(val, name, args) => {
      if let Some(e) = val {
        write_expr(gen, e);
        gen.write(".");
      }
      if name == "read_str" && args.is_empty() {
        gen.write("read_str(32767)");
      } else {
        gen.write(&name);
        gen.write("(");
        for (i, a) in args.iter().enumerate() {
          write_expr(gen, a);
          if i != args.len() - 1 {
            gen.write(", ");
          }
        }
        gen.write(")");
      }
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
    Cond::Less(lhs, rhs) => cond!(gen, lhs < rhs),
    Cond::Greater(lhs, rhs) => cond!(gen, lhs > rhs),
    Cond::Lte(lhs, rhs) => cond!(gen, lhs <= rhs),
    Cond::Gte(lhs, rhs) => cond!(gen, lhs >= rhs),

    Cond::Neq(lhs, rhs) => match &lhs.initial {
      // Matching `foo.equals("name") != 0`
      Value::Call(val, name, args) if name == "equals" && val.is_some() => {
        // dbg!(&lhs);
        assert_eq!(rhs, &Expr::new(Value::Lit(0.into())));
        assert_eq!(args.len(), 1);
        assert!(val.as_ref().unwrap().ops.is_empty());
        assert!(args[0].ops.is_empty());
        write_expr(gen, val.as_ref().unwrap());
        gen.write(" == ");
        write_val(gen, &args[0].initial);
      }
      _ => {
        cond!(gen, lhs != rhs)
      }
    },

    Cond::Or(lhs, rhs) => {
      gen.write("(");
      write_cond(gen, lhs);
      gen.write(") || (");
      write_cond(gen, rhs);
      gen.write(")");
    }
  }
}

/// Returns `(initialized, option)`. The first bool will be false when the field
/// needs a default value.
fn check_option(instr: &[Instr], field: &str) -> (bool, bool) {
  for i in instr {
    match i {
      Instr::Set(f, val) => {
        if val.initial == Value::Null {
          return (true, true);
        }
        if field == f {
          return (true, false);
        }
      }
      Instr::If(_, when_true, when_false) => {
        let mut assigned_true = false;
        let mut assigned_false = false;
        let mut needs_option = false;
        for i in when_true {
          match i {
            Instr::Set(f, val) => {
              if val.initial == Value::Null {
                needs_option = true;
              }
              if field == f {
                assigned_true = true;
                break;
              }
            }
            _ => {}
          }
        }
        for i in when_false {
          match i {
            Instr::Set(f, val) => {
              if val.initial == Value::Null {
                needs_option = true;
              }
              if field == f {
                assigned_false = true;
                break;
              }
            }
            _ => {}
          }
        }
        match (assigned_true, assigned_false) {
          (true, true) => return (true, needs_option),
          (false, true) => return (false, true),
          (true, false) => return (false, true),
          (false, false) => continue,
        }
      }
      _ => {}
    }
  }
  (false, true)
}
