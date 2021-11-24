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
        simplify_expr(arr);
        simplify_val(idx);
        simplify_expr(val);
      }
      Instr::Let(_, val) => simplify_expr(val),
      Instr::Expr(v) => simplify_expr(v),
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
    Value::CallStatic(_class, name, args) => {
      simplify_name(name);
      let new_name = convert::static_call(&name);
      *name = new_name.into();
      args.iter_mut().for_each(|a| simplify_expr(a))
    }
    Value::Closure(args, instr) => {
      for a in args.iter_mut() {
        simplify_expr(a);
      }
      simplify_instr(instr);
    }
    Value::New(_, args) => {
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
    Op::Field(_) => {}

    Op::If(cond, val) => {
      simplify_cond(cond);
      simplify_expr(val)
    }
    Op::Call(_name, args) => {
      for a in args.iter_mut() {
        simplify_expr(a);
      }
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
  let mut writer = InstrWriter::new(gen, &mut p2);
  for i in &p.reader {
    writer.write_instr(i);
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
          gen.write(convert::ty(read, &rs));
          gen.write(")");
        } else {
          gen.write(convert::ty(read, &rs));
        }
      }
    }
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("}");
}

struct InstrWriter<'a> {
  gen: &'a mut CodeGen,
  p:   &'a mut Packet,
}

impl<'a> InstrWriter<'a> {
  pub fn new(gen: &'a mut CodeGen, p: &'a mut Packet) -> Self {
    InstrWriter { gen, p }
  }
  pub fn write_instr(&mut self, instr: &Instr) {
    match instr {
      Instr::Super => {
        self.gen.write_comment("call super here");
      }
      Instr::Set(name, val) => {
        self.gen.write("f_");
        self.gen.write(&name);
        self.gen.write(" = ");
        if let Some(field) = self.p.get_field_mut(&name) {
          match &val.initial {
            // Conditionals as ops are always something like `if cond { 1 } else { 0 }`, which we
            // can convert with `v != 0`. So, in order to recognize that, we need to the
            // reader type to be a number.
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
        if self.p.get_field(&name).map(|f| f.option).unwrap_or(false) && val.initial != Value::Null
        {
          self.gen.write("Some(");
          self.write_expr(val);
          self.gen.write(")");
        } else {
          self.write_expr(val);
        }
        self.gen.write_line(";");
      }
      Instr::SetArr(arr, idx, val) => {
        self.write_expr(arr);
        self.gen.write("[");
        self.write_val(idx);
        self.gen.write("] = ");
        self.write_expr(val);
        self.gen.write_line(";");
      }
      Instr::Let(var, val) => {
        self.gen.write("let v_");
        self.gen.write(&var.to_string());
        self.gen.write(" = ");
        self.write_expr(val);
        self.gen.write_line(";");
      }
      Instr::Expr(v) => self.write_expr(v),
      Instr::If(cond, true_block, false_block) => {
        self.gen.write("if ");
        self.write_cond(cond);
        self.gen.write_line(" {");
        self.gen.add_indent();
        for i in true_block {
          self.write_instr(i);
        }
        self.gen.remove_indent();
        if !false_block.is_empty() {
          self.gen.write_line("} else {");
          self.gen.add_indent();
          for i in false_block {
            self.write_instr(i);
          }
          self.gen.remove_indent();
        }
        self.gen.write_line("}");
      }
      Instr::For(v, range, block) => {
        self.gen.write("for ");
        if let Var::Local(v) = v {
          self.gen.write("v_");
          self.gen.write(&v.to_string());
        } else {
          panic!("cannot iterate with self or buf as the value");
        }
        self.gen.write(" in ");
        self.write_expr(&range.min);
        self.gen.write("..");
        self.write_expr(&range.max);
        self.gen.write_line(" {");
        self.gen.add_indent();
        for i in block {
          self.write_instr(i);
        }
        self.gen.remove_indent();
        self.gen.write_line("}");
      }
      Instr::Switch(v, items) => {
        self.gen.write("match ");
        self.write_expr(v);
        self.gen.write(" ");
        let p = &mut self.p;
        self.gen.write_block(|gen| {
          for (key, instr) in items {
            gen.write(&key.to_string());
            gen.write(" => ");
            gen.write_block(|gen| {
              let mut w = InstrWriter::new(gen, p);
              for i in instr {
                w.write_instr(i);
              }
            });
          }
        });
      }
      Instr::CheckStrLen(val, len) => {
        self.gen.write("assert!(");
        self.write_expr(val);
        self.gen.write(".len() < ");
        self.write_val(len);
        self.gen.write(", \"string is too long (len greater than `");
        self.write_val(len);
        self.gen.write("`)\");");
      }
    }
  }

  fn write_expr(&mut self, e: &Expr) {
    let mut g = CodeGen::new();
    g.set_indent(self.gen.indent());
    {
      let mut inner = InstrWriter::new(&mut g, self.p);
      inner.write_val(&e.initial);
    }
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
      g.set_indent(self.gen.indent());
      {
        let mut i = InstrWriter::new(&mut g, self.p);
        if needs_paren {
          i.gen.write("(");
        }
        match op {
          Op::BitAnd(rhs) => {
            i.gen.write(&val);
            i.gen.write(" & ");
            i.write_expr(rhs);
          }
          Op::Shr(rhs) => {
            i.gen.write(&val);
            i.gen.write(" >> ");
            i.write_expr(rhs);
          }
          Op::UShr(rhs) => {
            i.gen.write(&val);
            i.gen.write(" >> ");
            i.write_expr(rhs);
          }
          Op::Shl(rhs) => {
            i.gen.write(&val);
            i.gen.write(" << ");
            i.write_expr(rhs);
          }

          Op::Add(rhs) => {
            i.gen.write(&val);
            i.gen.write(" + ");
            i.write_expr(rhs);
          }
          Op::Div(rhs) => {
            i.gen.write(&val);
            i.gen.write(" / ");
            i.write_expr(rhs);
          }

          Op::Len => {
            i.gen.write(&val);
            i.gen.write(".len()");
          }
          Op::Idx(rhs) => {
            i.gen.write(&val);
            i.gen.write("[");
            i.write_expr(rhs);
            i.gen.write(".try_into().unwrap()]");
          }
          Op::Field(name) => {
            i.gen.write(&val);
            i.gen.write(".");
            i.gen.write(&name);
          }

          Op::If(cond, new) => {
            i.gen.write("if ");
            i.write_cond(cond);
            i.gen.write(" { ");
            i.gen.write(&val);
            i.gen.write(" } else { ");
            if e.initial == Value::Null {
              i.gen.write("Some(");
              i.write_expr(new);
              i.gen.write(")");
            } else {
              i.write_expr(new);
            }
            i.gen.write(" }");
          }

          Op::Call(name, args) => {
            i.gen.write(".");
            i.gen.write(&name);
            i.gen.write("(");
            for (idx, a) in args.iter().enumerate() {
              i.write_expr(a);
              if idx != args.len() - 1 {
                i.gen.write(", ");
              }
            }
            i.gen.write(")");
          }
        }
        if needs_paren {
          i.gen.write(")");
        }
      }
      val = g.into_output();
    }
    self.gen.write(&val);
  }

  fn write_val(&mut self, val: &Value) {
    match val {
      Value::Null => self.gen.write("None"),
      Value::Lit(lit) => match lit {
        Lit::Int(v) => self.gen.write(&v.to_string()),
        Lit::Float(v) => self.gen.write(&v.to_string()),
        Lit::String(v) => {
          self.gen.write("\"");
          self.gen.write(&v);
          self.gen.write("\"");
        }
      },
      Value::Var(v) => match v {
        Var::This => self.gen.write("self"),
        Var::Buf => self.gen.write("p"),
        Var::Local(v) => {
          self.gen.write("v_");
          self.gen.write(&v.to_string())
        }
      },
      Value::Static(class, name) => {
        for s in class.split('/').last().unwrap().split('$') {
          self.gen.write(s);
        }
        self.gen.write(".");
        self.gen.write(name);
      }
      Value::Field(name) => {
        self.gen.write("f_");
        self.gen.write(name);
      }
      Value::Array(len) => {
        self.gen.write("Vec::with_capacity(");
        self.write_expr(len);
        self.gen.write(".try_into().unwrap())");
      }
      Value::CallStatic(class, name, args) => {
        if name == "read_str" && args.is_empty() {
          self.gen.write("read_str(32767)");
        } else {
          self.gen.write(&name);
          self.gen.write("(");
          for (i, a) in args.iter().enumerate() {
            self.write_expr(a);
            if i != args.len() - 1 {
              self.gen.write(", ");
            }
          }
          self.gen.write(")");
        }
      }
      Value::Closure(args, instr) => {
        self.gen.write("|");
        for (i, a) in args.iter().enumerate() {
          self.write_expr(a);
          if i != args.len() - 1 {
            self.gen.write(", ");
          }
        }
        self.gen.write_line("| {");
        self.gen.add_indent();
        for i in instr {
          self.write_instr(i);
        }
        self.gen.remove_indent();
        self.gen.write("}");
      }
      Value::New(name, args) => {
        self.gen.write(name.split('/').last().unwrap().split('$').last().unwrap());
        self.gen.write("::new(");
        for (i, a) in args.iter().enumerate() {
          self.write_expr(a);
          if i != args.len() - 1 {
            self.gen.write(", ");
          }
        }
        self.gen.write(")");
      }
    }
  }

  fn write_cond(&mut self, cond: &Cond) {
    macro_rules! cond {
      ($lhs:ident $comp:tt $rhs:ident) => {{
        self.write_expr($lhs);
        self.gen.write(concat!(" ", stringify!($comp), " "));
        self.write_expr($rhs);
      }};
    }
    match cond {
      Cond::Eq(lhs, rhs) => cond!(lhs == rhs),
      Cond::Less(lhs, rhs) => cond!(lhs < rhs),
      Cond::Greater(lhs, rhs) => cond!(lhs > rhs),
      Cond::Lte(lhs, rhs) => cond!(lhs <= rhs),
      Cond::Gte(lhs, rhs) => cond!(lhs >= rhs),

      Cond::Neq(lhs, rhs) => match &lhs.initial {
        // Matching `foo.equals("name") != 0`
        // Value::CallStatic(class, name, args) if name == "equals" && val.is_some() => {
        //   // dbg!(&lhs);
        //   assert_eq!(rhs, &Expr::new(Value::Lit(0.into())));
        //   assert_eq!(args.len(), 1);
        //   assert!(val.as_ref().unwrap().ops.is_empty());
        //   assert!(args[0].ops.is_empty());
        //   self.write_expr(val.as_ref().unwrap());
        //   self.gen.write(" == ");
        //   self.write_val(&args[0].initial);
        // }
        // Matching `equals(var, foo) != 0`
        Value::CallStatic(_class, name, args) if name == "equals" => {
          // dbg!(&lhs);
          assert_eq!(rhs, &Expr::new(Value::Lit(0.into())));
          assert_eq!(args.len(), 2);
          assert!(args[0].ops.is_empty());
          assert!(args[1].ops.is_empty());
          self.write_val(&args[0].initial);
          self.gen.write(" == ");
          self.write_val(&args[1].initial);
        }
        _ => {
          cond!(lhs != rhs)
        }
      },

      Cond::Or(lhs, rhs) => {
        self.gen.write("(");
        self.write_cond(lhs);
        self.gen.write(") || (");
        self.write_cond(rhs);
        self.gen.write(")");
      }
    }
  }
}

/// Returns `(initialized, option)`. The first bool will be false when the
/// field needs a default value.
fn check_option(instr: &[Instr], field: &str) -> (bool, bool) {
  for i in instr {
    match i {
      Instr::Set(f, val) => {
        if field == f {
          if val.initial == Value::Null {
            return (true, true);
          }
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
              if field == f {
                if val.initial == Value::Null {
                  needs_option = true;
                }
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
              if field == f {
                if val.initial == Value::Null {
                  needs_option = true;
                }
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
