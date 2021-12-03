use super::{
  convert, simplify, Cond, Expr, Field, Instr, Lit, Op, Packet, PacketDef, RType, Type, Value,
  VarKind,
};
use crate::{gen::CodeGen, Version};
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
#[derive(Debug)]
struct PacketCollection {
  packets: HashMap<String, Vec<(Version, Packet)>>,
}

impl PacketCollection {
  pub fn new() -> Self {
    PacketCollection { packets: HashMap::new() }
  }
  pub fn add(&mut self, ver: Version, mut p: Packet) {
    simplify::pass(&mut p);
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
    let mut packets: Vec<Vec<(_, _)>> = packets.into_iter().map(|(_, v)| v).collect();
    for versions in &mut packets {
      for (v, p) in versions {
        println!("finding reader type of {} for ver {}", p.name, v);
        // dbg!(&p);
        p.find_reader_types();
      }
    }

    gen.write_line("use crate::{");
    gen.write_line("  ChunkPos, Pos,");
    gen.write_line("  version::ProtocolVersion,");
    gen.write_line("  util::{Item, nbt::NBT, UUID},");
    gen.write_line("};");
    gen.write_line("use std::collections::{HashMap, HashSet};");
    gen.write_line("");
    gen.write_line("pub struct U;");
    gen.write_line("");

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
                gen.write(&ver.to_protocol());
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
                    gen.write(&next_ver.0.to_protocol());
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

fn write_packet(gen: &mut CodeGen, name: &str, p: &Packet) {
  gen.write(name);
  gen.write_line(" {");
  gen.add_indent();
  for f in &p.fields {
    gen.write(&f.name);
    gen.write(": ");
    if f.option {
      gen.write("Option<");
      gen.write(&f.ty.to_rust().to_string());
      gen.write(">");
    } else {
      gen.write(&f.ty.to_rust().to_string());
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
      if matches!(f.ty, Type::Array(_)) {
        gen.write(" = vec![]");
      } else {
        gen.write(" = None");
      }
    }
    gen.write_line(";");
  }
  let mut p2 = p.clone();
  let mut writer = InstrWriter::new(gen, &mut p2);
  for i in &p.reader.block {
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
      // if &rs != read {
      //   if f.option {
      //     gen.write(".map(|v| v");
      //     gen.write(convert::ty(read, &rs));
      //     gen.write(")");
      //   } else {
      //     gen.write(convert::ty(read, &rs));
      //   }
      // }
    }
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("}");
}

#[derive(Debug)]
struct InstrWriter<'a> {
  gen:        &'a mut CodeGen,
  fields:     &'a mut Vec<Field>,
  vars:       &'a Vec<VarKind>,
  is_closure: bool,
}

impl<'a> InstrWriter<'a> {
  pub fn new(gen: &'a mut CodeGen, p: &'a mut Packet) -> Self {
    InstrWriter { gen, fields: &mut p.fields, vars: &p.reader.vars, is_closure: false }
  }
  fn new_inner(gen: &'a mut CodeGen, fields: &'a mut Vec<Field>, vars: &'a Vec<VarKind>) -> Self {
    InstrWriter { gen, fields, vars, is_closure: false }
  }
  pub fn write_instr(&mut self, instr: &Instr) {
    match instr {
      Instr::Super => {
        self.gen.write_comment("call super here");
      }
      Instr::Set(f_name, val) => {
        // Terrible hack. Only applies to 1.8. This was too ugly to implement correctly.
        if f_name == "chunks_data.data_size" {
          self.gen.write_line("let len = (p.read_i16() & 65535).try_into().unwrap();");
        } else if f_name == "chunks_data.data" {
          self.gen.write_line("f_chunks_data.push(p.read_buf(len));");
        } else {
          self.gen.write("f_");
          self.gen.write(&f_name);
          self.gen.write(" = ");
          if let Some(field) = self.get_field_mut(&f_name) {
            match &val.initial {
              Value::Var(1)
                if !val.ops.is_empty()
                  && val.ops.first().map(|op| matches!(op, Op::Call(..))).unwrap_or(false) =>
              {
                let (name, _args) = match val.ops.first().unwrap() {
                  Op::Call(_, name, args) => (name, args),
                  _ => unreachable!(),
                };
                let ty = convert::reader_func_to_ty(&f_name, name);
                if let Some(ref reader) = field.reader_type {
                  assert_eq!(reader, &ty);
                } else {
                  field.reader_type = Some(ty.into());
                }
              }
              Value::Lit(lit) => {
                let ty = match lit {
                  Lit::Int(_) => RType::new("i32"),
                  Lit::Float(_) => RType::new("f32"),
                  Lit::String(_) => RType::new("String"),
                };
                if let Some(ref reader) = field.reader_type {
                  assert_eq!(reader, &ty);
                } else {
                  field.reader_type = Some(ty.into());
                }
              }
              // Conditionals as ops are always something like `if cond { 1 } else { 0 }`, which we
              // can convert with `v != 0`. So, in order to recognize that, we need to the
              // reader type to be a number.
              _ if matches!(&val.ops.last(), Some(Op::If(_, _))) => {
                // if let Some(ref reader) = field.reader_type {
                //   assert_eq!(reader, "u8");
                // } else {
                //   field.reader_type = Some("u8".into());
                // }
              }
              _ => {}
            }
          }
          if self.get_field(&f_name).map(|f| f.option).unwrap_or(false)
            && val.initial != Value::Null
          {
            self.gen.write("Some(");
            self.write_expr(val);
            self.gen.write(")");
          } else {
            self.write_expr(val);
          }
          self.gen.write_line(";");
        }
      }
      Instr::SetArr(arr, _idx, val) => {
        // 1.8 hack. This is too terrible to implement correctly.
        if arr.initial == Value::Field("chunks_data".into()) {
          return;
        }
        // This might break things. However, everything single SetArr I can find has
        // just used the for loop value in the index. So, I am going to go ahead and
        // assume thats always the case. Modern versions don't use for loops very much
        // at all, and they never use SetArr. So if this works now, it should continue
        // working in the future.
        self.write_expr(arr);
        self.gen.write(".push(");
        // self.gen.write("[");
        // self.write_val(idx);
        // self.gen.write(".try_into().unwrap()] = ");
        self.write_expr(val);
        self.gen.write_line(");");
      }
      Instr::Let(var, val) => {
        self.gen.write("let v_");
        self.gen.write(&var.to_string());
        self.gen.write(" = ");
        self.write_expr(val);
        self.gen.write_line(";");
      }
      Instr::Expr(v) => {
        self.write_expr(v);
        self.gen.write_line(";")
      }
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
        self.gen.write("for v_");
        self.gen.write(&v.to_string());
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
        let fields = &mut self.fields;
        let vars = &self.vars;
        self.gen.write_block(|gen| {
          for (key, instr) in items {
            gen.write(&key.to_string());
            gen.write(" => ");
            gen.write_block(|gen| {
              let mut w = InstrWriter::new_inner(gen, fields, vars);
              w.is_closure = self.is_closure;
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
      Instr::Return(v) => {
        if self.is_closure {
          self.gen.write("return ");
          self.write_expr(v);
          self.gen.write_line(";");
        }
      }
    }
  }

  fn write_expr(&mut self, e: &Expr) {
    if e.ops.last() == Some(&Op::Field("type".into())) {
      self.gen.write("f_ty");
      return;
    }
    let mut g = CodeGen::new();
    g.set_indent(self.gen.indent());
    {
      let mut inner = InstrWriter::new_inner(&mut g, &mut self.fields, &self.vars);
      inner.is_closure = self.is_closure;
      inner.write_val(&e.initial);
    }
    if !e.ops.is_empty()
      && matches!(&e.initial, Value::Field(field) if self.get_field(&field).map(|v| v.option).unwrap_or(false))
    {
      g.write(".as_mut().unwrap()");
    }
    let mut val = g.into_output();
    for (i, op) in e.ops.iter().enumerate() {
      let needs_paren =
        e.ops.get(i + 1).map(|next| next.precedence() < op.precedence()).unwrap_or(false);
      let mut g = CodeGen::new();
      g.set_indent(self.gen.indent());
      {
        let mut i = InstrWriter::new_inner(&mut g, &mut self.fields, &self.vars);
        i.is_closure = self.is_closure;
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
            if matches!(rhs.initial, Value::Lit(Lit::Float(_))) {
              i.gen.write(" as f32");
            }
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

          Op::Call(_class, name, args) => {
            i.gen.write(&val);
            if !(name == "get" && args.len() == 0) {
              i.gen.write(".");
              if name == "read_str" && args.is_empty() {
                i.gen.write("read_str(32767)");
              } else if name == "read_byte_arr" && args.len() == 1 {
                i.gen.write("read_byte_arr_max(");
                for a in args.iter() {
                  i.write_expr(a);
                }
                i.gen.write(")");
              } else if name == "read_map" && args.len() == 3 {
                i.gen.write("read_map(");
                for (idx, a) in args.iter().enumerate().skip(1) {
                  i.write_expr(a);
                  if idx != args.len() - 1 {
                    i.gen.write(", ");
                  }
                }
                i.gen.write(")");
              } else if name == "read_collection" && args.len() == 2 {
                let mut args = args.clone();
                match &args[0].initial {
                  Value::MethodRef(class, name)
                    if class == "HashSet" && name == "with_capacity" =>
                  {
                    i.gen.write("read_set(");
                  }
                  Value::MethodRef(class, name) if class == "Vec" && name == "with_capacity" => {
                    i.gen.write("read_list(");
                  }
                  Value::CallStatic(class, name, inner_args)
                    if class == "tcp::Packet" && name == "get_max_validator" =>
                  {
                    assert!(inner_args.len() == 2, "{:?}", args);
                    let len = inner_args[1].clone();
                    match &inner_args[0].initial {
                      Value::MethodRef(class, name)
                        if class == "com/google/common/collect/Sets"
                          && (name == "new_linked_hash_set_with_expected_size"
                            || name == "new_hash_set_with_expected_size") =>
                      {
                        i.gen.write("read_set_max(");
                        args.push(len);
                      }
                      Value::MethodRef(class, name)
                        if class == "com/google/common/collect/Lists"
                          && name == "new_array_list_with_capacity" =>
                      {
                        i.gen.write("read_list_max(");
                        args.push(len);
                      }
                      _ => panic!("unexpected read_collection args {:?}", inner_args),
                    }
                  }
                  _ => panic!("unexpected read_collection args {:?}", args),
                }
                for (idx, a) in args.iter().enumerate().skip(1) {
                  i.write_expr(a);
                  if idx != args.len() - 1 {
                    i.gen.write(", ");
                  }
                }
                i.gen.write(")");
              } else {
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
          }

          Op::Cast(ty) => {
            i.gen.write(&val);
            i.gen.write(match ty {
              Type::Byte => " as i8",
              Type::Short => " as i16",
              Type::Int => " as i32",
              Type::Long => " as i64",
              Type::Float => " as f32",
              Type::Double => " as f64",
              _ => unreachable!(),
            });
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
        Lit::Float(v) => {
          self.gen.write(&v.to_string());
          if v.fract() == 0.0 {
            self.gen.write(".0");
          }
        }
        Lit::String(v) => {
          self.gen.write("\"");
          self.gen.write(&v);
          self.gen.write("\"");
        }
      },
      Value::Var(v) => self.write_var(*v),
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
        self.gen.write(class.split('/').last().unwrap().split('$').last().unwrap());
        self.gen.write("::");
        if name == "<init>" {
          self.gen.write("new");
        } else {
          self.gen.write(&name);
        }
        self.gen.write("(");
        for (i, a) in args.iter().enumerate() {
          self.write_expr(a);
          if i != args.len() - 1 {
            self.gen.write(", ");
          }
        }
        self.gen.write(")");
      }
      Value::MethodRef(class, name) => {
        self.gen.write(class.split('/').last().unwrap().split('$').last().unwrap());
        self.gen.write("::");
        if name == "<init>" {
          self.gen.write("new");
        } else {
          self.gen.write(&name);
        }
      }
      Value::Closure(_args, block) => {
        // self.gen.write("|");
        // for (i, a) in args.iter().enumerate() {
        //   self.write_expr(a);
        //   if i != args.len() - 1 {
        //     self.gen.write(", ");
        //   }
        // }
        self.gen.write_line("|buf| {");
        self.gen.add_indent();
        {
          let mut inner = InstrWriter::new_inner(&mut self.gen, &mut self.fields, &block.vars);
          inner.is_closure = true;
          for i in &block.block {
            inner.write_instr(i);
          }
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

  fn write_var(&mut self, v: usize) {
    match self.vars[v] {
      VarKind::This => self.gen.write("self"),
      VarKind::Arg => self.gen.write("p"),
      VarKind::Local => {
        self.gen.write("v_");
        self.gen.write(&v.to_string());
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
        _ if matches!(lhs.ops.get(0), Some(Op::Call(_, name, _)) if name == "equals") => {
          let args = match &lhs.ops[0] {
            Op::Call(_, _, args) => args,
            _ => unreachable!(),
          };
          // dbg!(&lhs);
          assert_eq!(rhs, &Expr::new(Value::Lit(0.into())));
          assert_eq!(args.len(), 1);
          assert!(args[0].ops.is_empty());
          self.write_val(&lhs.initial);
          self.gen.write(" == ");
          self.write_val(&args[0].initial);
        }
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
  pub fn get_field(&self, name: &str) -> Option<&Field> {
    for f in self.fields.iter() {
      if f.name == name {
        return Some(f);
      }
    }
    None
  }
  pub fn get_field_mut(&mut self, name: &str) -> Option<&mut Field> {
    for f in self.fields.iter_mut() {
      if f.name == name {
        return Some(f);
      }
    }
    None
  }
}
