use super::{
  convert, simplify, Cond, Expr, Field, Instr, Lit, Op, Packet, PacketDef, Type, Value, VarKind,
};
use crate::{
  gen::{CodeGen, FuncArg},
  Version,
};
use std::{collections::HashMap, fs, fs::File, io, io::Write, path::Path};

pub fn generate(def: Vec<(Version, PacketDef)>, dir: &Path) -> io::Result<()> {
  let mut all_cb_packets = PacketCollection::new();
  let mut all_sb_packets = PacketCollection::new();

  for (ver, def) in def {
    for (i, p) in def.clientbound.into_iter().enumerate() {
      all_cb_packets.add(ver, p, i as i32);
    }
    for (i, p) in def.serverbound.into_iter().enumerate() {
      all_sb_packets.add(ver, p, i as i32);
    }
  }

  all_cb_packets.expand_sup();
  all_sb_packets.expand_sup();

  all_cb_packets.finish_simplify();
  all_sb_packets.finish_simplify();

  fs::create_dir_all(dir)?;
  File::create(dir.join("cb.rs"))?.write_all(all_cb_packets.gen().as_bytes())?;
  File::create(dir.join("sb.rs"))?.write_all(all_sb_packets.gen().as_bytes())?;

  Ok(())
}
#[derive(Debug)]
struct PacketCollection {
  // Maps packet names to [version, packet]
  packets:  HashMap<String, Vec<(Version, Packet)>>,
  // Maps versions to packet class -> packet
  classes:  HashMap<Version, HashMap<String, Packet>>,
  // Maps versions to packet name -> tcp id
  versions: HashMap<Version, HashMap<String, i32>>,
}

impl PacketCollection {
  pub fn new() -> Self {
    PacketCollection {
      packets:  HashMap::new(),
      classes:  crate::VERSIONS.iter().map(|v| (*v, HashMap::new())).collect(),
      versions: crate::VERSIONS.iter().map(|v| (*v, HashMap::new())).collect(),
    }
  }
  pub fn add(&mut self, ver: Version, mut p: Packet, tcp_id: i32) {
    simplify::pass(&mut p);
    self.classes.get_mut(&ver).unwrap().insert(p.class.clone(), p.clone());
    self.versions.get_mut(&ver).unwrap().insert(p.name.clone(), tcp_id);
    let list = self.packets.entry(p.name.clone()).or_insert_with(|| vec![]);
    if let Some((_, last)) = list.last() {
      if *last == p {
        return;
      }
    }
    list.push((ver, p));
  }
  // This function is not complete, so we allow this here
  #[allow(clippy::if_same_then_else)]
  pub fn expand_sup(&mut self) {
    for (_name, versions) in &mut self.packets {
      for (ver, p) in versions {
        if p.extends == "Object" || p.extends == "Record" {
          p.extend_from_none();
        } else if (p.extends == "EntityS2CPacket" || p.extends == "PlayerMoveC2SPacket")
          && !self.classes[ver].contains_key(&p.extends)
        {
          // On 1.17+, the Entity packet is no longer registered, but they all
          // still extend from that class. However, in 1.17+, the reader
          // contains everything we need.
          //
          // TODO: We still might be missing fields from this packet.
          p.extend_from_none();
        } else {
          dbg!(&p);
          p.extend_from(&self.classes[ver][&p.extends]);
        }
      }
    }
  }
  pub fn finish_simplify(&mut self) {
    for versions in self.packets.values_mut() {
      for (_ver, p) in versions {
        simplify::finish(p);
      }
    }
  }
  pub fn gen(self) -> String {
    let mut gen = CodeGen::new();

    let mut packets: Vec<_> = self.packets.into_iter().collect();
    packets.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    let mut packets: Vec<Vec<(_, _)>> = packets.into_iter().map(|(_, v)| v).collect();
    for versions in &mut packets {
      for (_v, p) in versions {
        // eprintln!("finding reader type of {} for ver {}", p.name, v);
        // dbg!(&_v, &p);
        p.find_reader_types_gen_writer();
      }
    }

    gen.write_line("// Some imports are used on clientbound packets, but not on serverbound");
    gen.write_line("// packets. This is to remove those warnings.");
    gen.write_line("#[allow(unused_imports)]");
    gen.write_line("use sc_common::{");
    gen.write_line("  math::{ChunkPos, Pos},");
    gen.write_line("  version::ProtocolVersion,");
    gen.write_line("  util::{Item, UUID},");
    gen.write_line("  nbt::NBT,");
    gen.write_line("};");
    gen.write_line("#[allow(unused_imports)]");
    gen.write_line("use std::collections::{HashMap, HashSet};");
    gen.write_line("use sc_transfer::{");
    gen.write_line("  MessageRead, MessageReader, MessageWrite, MessageWriter, ReadError,");
    gen.write_line("  WriteError,");
    gen.write_line("};");
    gen.write_line("use crate::Error;");
    gen.write_line("");
    gen.write_line("#[derive(Debug, Clone, PartialEq, Eq, Hash)]");
    gen.write_line("pub struct U;");
    gen.write_line("");
    gen.write_line("impl MessageRead<'_> for U {");
    gen.write_line("  fn read(_: &mut MessageReader) -> Result<Self, ReadError> { Ok(U) }");
    gen.write_line("}");
    gen.write_line("impl MessageWrite for U {");
    gen.write_line("  fn write(&self, _: &mut MessageWriter) -> Result<(), WriteError> { Ok(()) }");
    gen.write_line("}");
    gen.write_line("");

    gen.write_line("#[derive(Debug, Clone)]");
    gen.write("pub enum Packet ");
    gen.write_block(|gen| {
      for versions in &packets {
        for (ver, p) in versions {
          write_packet(gen, &format!("{}V{}", p.name, ver.maj), p, *ver);
        }
      }
    });

    gen.write_impl("Packet", |gen| {
      gen.write("pub fn tcp_id(&self, ver: ProtocolVersion) -> u32 ");
      gen.write_block(|gen| {
        gen.write_match("ver.id()", |gen| {
          for match_ver in crate::VERSIONS {
            gen.write_comment(&match_ver.to_string());
            gen.write(&match_ver.protocol.to_string());
            gen.write(" => ");
            gen.write_match("self", |gen| {
              for versions in packets.iter() {
                for (ver, p) in versions.iter().rev() {
                  if ver.maj > match_ver.maj {
                    continue;
                  }
                  gen.write("Packet::");
                  gen.write(&p.name);
                  gen.write("V");
                  gen.write(&ver.maj.to_string());
                  gen.write(" { .. } => ");
                  let val = self.versions[match_ver].get(&p.name);
                  let id = val.unwrap_or(&0);
                  gen.write(&id.to_string());
                  gen.write(", // ");
                  gen.write(&format!("{:#x}", id));
                  if val.is_some() {
                    gen.write_line("");
                  } else {
                    gen.write_line(" (not found)");
                  }
                  break;
                }
              }
              gen.write_line(
                r#"_ => panic!("packet {:?} does not exist on version {}", self, ver)"#,
              );
            });
          }
          gen.write_line(r#"_ => panic!("unknown version {}", ver),"#);
        });
      });
      gen.write_line("#[allow(unused_mut, unused_variables)]");
      gen.write(
        "pub fn from_tcp(p: &mut tcp::Packet, ver: ProtocolVersion) -> Result<Self, Error> ",
      );
      gen.write_block(|gen| {
        gen.write("Ok(");
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
          gen.write_line(r#"v => panic!("invalid protocol version {}", v),"#);
        });
        gen.write(")"); // Close the `Ok(` from above the match
      });
      gen.write_line("#[allow(unused_mut, unused_variables, unused_assignments)]");
      gen.write("pub fn to_tcp(&self, p: &mut tcp::Packet) ");
      gen.write_block(|gen| {
        gen.write_match("self", |gen| {
          for (_id, versions) in packets.iter().enumerate() {
            for (ver, p) in versions.iter() {
              write_to_tcp(gen, p, *ver);
            }
          }
        });
      });
    });

    gen.write_line("/// Unreachable patterns mean the packet has been removed in that version.");
    gen.write_line("/// This is most likely because I messed up the naming. However, some packets");
    gen.write_line("/// have been removed in practice. TODO: Handle removed packets.");
    gen.write_line("#[allow(unreachable_patterns)]");
    gen.write_func(
      "to_sug_id",
      &[FuncArg { name: "id", ty: "i32" }, FuncArg { name: "ver", ty: "ProtocolVersion" }],
      Some("i32"),
      |gen| {
        gen.write_match("ver.id()", |gen| {
          for match_ver in crate::VERSIONS {
            gen.write_comment(&match_ver.to_string());
            gen.write(&match_ver.protocol.to_string());
            gen.write(" => ");
            gen.write_match("id", |gen| {
              for (sug_id, versions) in packets.iter().enumerate() {
                if let Some((ver, p)) = versions.first() {
                  if ver.maj <= match_ver.maj {
                    if let Some(tcp_id) = self.versions[match_ver].get(&p.name) {
                      gen.write(&tcp_id.to_string());
                      gen.write(" => ");
                      gen.write(&sug_id.to_string());
                      gen.write(", // ");
                      gen.write_line(&p.name);
                    }
                  }
                }
              }
              gen.write_line("_ => 0,");
            });
          }
          gen.write_line("_ => 0,");
        });
      },
    );

    gen.into_output()
  }
}

fn write_packet(gen: &mut CodeGen, name: &str, p: &Packet, ver: Version) {
  gen.set_doc_comment(true);
  gen.write_line("Definition:");
  gen.write_line("```rust,ignore");
  write_def(gen, name, p);
  gen.write_line("```");

  gen.write_line("Both these code blocks use [`tcp::Packet`](crate::gnet::tcp::Packet) for");
  gen.write_line("reading/writing fields.");
  gen.write_line("");

  gen.write_line("[`from_tcp`](Self::from_tcp) (packet reader):");
  gen.write_line("```rust,ignore");
  gen.write_line("let p = tcp::Packet::new(vec![]);");
  gen.write_line("");
  write_from_tcp(gen, p, ver);
  gen.write_line("```");
  gen.write_line("");

  gen.write_line("[`to_tcp`](Self::to_tcp) (packet writer):");
  gen.write_line("```rust,ignore");
  gen.write_line("let p = tcp::Packet::new(vec![]);");
  gen.write_line("");
  gen.write_line("match self {");
  gen.add_indent();
  write_to_tcp(gen, p, ver);
  gen.write_line("_ => { /* ... */ }");
  gen.remove_indent();
  gen.write_line("}");
  gen.write_line("```");
  gen.set_doc_comment(false);

  write_def(gen, name, p);
}

fn write_def(gen: &mut CodeGen, name: &str, p: &Packet) {
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

pub fn write_from_tcp(gen: &mut CodeGen, p: &Packet, ver: Version) {
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
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("}");
}
pub fn write_to_tcp(gen: &mut CodeGen, p: &Packet, ver: Version) {
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
    gen.write_line(",");
  }
  gen.remove_indent();
  gen.write_line("} => {");
  gen.add_indent();

  let mut p2 = p.clone();
  let mut writer = InstrWriter::new(gen, &mut p2);
  writer.needs_deref = true;
  for i in &p.writer.block {
    writer.write_instr(i);
  }

  gen.remove_indent();
  gen.write_line("}");
}

#[derive(Debug)]
struct InstrWriter<'a> {
  gen:         &'a mut CodeGen,
  fields:      &'a mut Vec<Field>,
  vars:        &'a [VarKind],
  is_closure:  bool,
  needs_deref: bool,
}

impl<'a> InstrWriter<'a> {
  pub fn new(gen: &'a mut CodeGen, p: &'a mut Packet) -> Self {
    InstrWriter {
      gen,
      fields: &mut p.fields,
      vars: &p.reader.vars,
      is_closure: false,
      needs_deref: false,
    }
  }
  fn new_inner(gen: &'a mut CodeGen, fields: &'a mut Vec<Field>, vars: &'a [VarKind]) -> Self {
    InstrWriter { gen, fields, vars, is_closure: false, needs_deref: false }
  }
  pub fn write_instr(&mut self, instr: &Instr) {
    match instr {
      Instr::Super => {
        self.gen.write_comment("call super here");
      }
      Instr::Set(f_name, val) => {
        let mut val = val.clone();
        // Terrible hack. Only applies to 1.8. This was too ugly to implement correctly.
        if f_name == "chunks_data.data_size" {
          self.gen.write_line("let len = (p.read_i16() & 65535).try_into().unwrap();");
        } else if f_name == "chunks_data.data" {
          self.gen.write_line("f_chunks_data.push(p.read_buf(len));");
        } else {
          self.gen.write("f_");
          self.gen.write(f_name);
          self.gen.write(" = ");
          if let Some(field) = self.get_field(f_name) {
            let ty = field.ty.to_rust();
            if let Some(ref reader) = field.reader_type {
              if *reader != ty {
                val.ops.extend(convert::type_cast(reader, &ty));
              }
            }
          }
          if self.get_field(f_name).map(|f| f.option).unwrap_or(false) && val.initial != Value::Null
          {
            self.gen.write("Some(");
            self.write_expr(&val);
            self.gen.write(")");
          } else {
            self.write_expr(&val);
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
        self.gen.write("let mut v_");
        self.gen.write(&var.to_string());
        self.gen.write(" = ");
        self.write_expr(val);
        self.gen.write_line(";");
      }
      Instr::SetVar(var, val) => {
        self.gen.write("v_");
        self.gen.write(&var.to_string());
        self.gen.write(" = ");
        self.write_expr(val);
        self.gen.write_line(";");
      }
      Instr::SetVarOr(var, val) => {
        self.gen.write("v_");
        self.gen.write(&var.to_string());
        self.gen.write(" |= ");
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
              w.needs_deref = self.needs_deref;
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
      let mut inner = InstrWriter::new_inner(&mut g, self.fields, self.vars);
      inner.is_closure = self.is_closure;
      inner.needs_deref = self.needs_deref;
      inner.write_val(&e.initial);
    }
    let mut val = g.into_output();
    for (i, op) in e.ops.iter().enumerate() {
      let needs_paren =
        e.ops.get(i + 1).map(|next| next.precedence() < op.precedence()).unwrap_or(false);
      let mut g = CodeGen::new();
      g.set_indent(self.gen.indent());
      {
        let mut i = InstrWriter::new_inner(&mut g, self.fields, self.vars);
        i.is_closure = self.is_closure;
        i.needs_deref = self.needs_deref;
        if needs_paren {
          i.gen.write("(");
        }
        i.write_op(&val, op);
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
        Lit::Int(v) => {
          self.gen.write(&v.to_string());
          if *v > 9 {
            self.gen.write(&format!(" /* {:#x} */", v));
          }
        }
        Lit::Float(v) => {
          self.gen.write(&v.to_string());
          if v.fract() == 0.0 {
            self.gen.write(".0");
          }
        }
        Lit::String(v) => {
          self.gen.write("\"");
          self.gen.write(v);
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
          self.gen.write(name);
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
          self.gen.write(name);
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
          let mut inner = InstrWriter::new_inner(self.gen, self.fields, &block.vars);
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
      Value::Cond(cond) => {
        self.write_cond(cond);
      }
    }
  }

  fn write_op(&mut self, val: &str, op: &Op) {
    match op {
      Op::BitAnd(rhs) => {
        self.gen.write(val);
        self.gen.write(" & ");
        self.write_expr(rhs);
      }
      Op::BitOr(rhs) => {
        self.gen.write(val);
        self.gen.write(" | ");
        self.write_expr(rhs);
      }
      Op::Shr(rhs) => {
        self.gen.write(val);
        self.gen.write(" >> ");
        self.write_expr(rhs);
      }
      Op::UShr(rhs) => {
        self.gen.write(val);
        self.gen.write(" >> ");
        self.write_expr(rhs);
      }
      Op::Shl(rhs) => {
        self.gen.write(val);
        self.gen.write(" << ");
        self.write_expr(rhs);
      }

      Op::Add(rhs) => {
        self.gen.write(val);
        self.gen.write(" + ");
        self.write_expr(rhs);
      }
      Op::Sub(rhs) => {
        self.gen.write(val);
        self.gen.write(" - ");
        self.write_expr(rhs);
      }
      Op::Div(rhs) => {
        self.gen.write(val);
        self.gen.write(" / ");
        self.write_expr(rhs);
      }
      Op::Mul(rhs) => {
        self.gen.write(val);
        self.gen.write(" * ");
        self.write_expr(rhs);
      }

      Op::Deref => {
        self.gen.write("*");
        self.gen.write(val);
      }
      Op::Not => {
        self.gen.write("!");
        self.gen.write(val);
      }
      Op::Try => {
        self.gen.write(val);
        self.gen.write("?");
      }
      Op::Len => {
        self.gen.write(val);
        self.gen.write(".len()");
      }
      Op::Idx(rhs) => {
        self.gen.write(val);
        self.gen.write("[");
        self.write_expr(rhs);
        self.gen.write(".try_into().unwrap()]");
      }
      Op::Field(name) => {
        self.gen.write(val);
        self.gen.write(".");
        self.gen.write(name);
      }

      Op::If(cond, new) => {
        self.gen.write("if ");
        self.write_cond(cond);
        self.gen.write(" { ");
        self.gen.write(val);
        self.gen.write(" } else { ");
        if new.initial == Value::Null {
          self.gen.write("Some(");
          self.write_expr(new);
          self.gen.write(")");
        } else {
          self.write_expr(new);
        }
        self.gen.write(" }");
      }

      Op::WrapCall(class, name, args) => {
        self.gen.write(class);
        self.gen.write("::");
        self.gen.write(name);
        self.gen.write("(");
        self.gen.write(val);
        if !args.is_empty() {
          self.gen.write(", ");
        }
        for (idx, a) in args.iter().enumerate() {
          self.write_expr(a);
          if idx != args.len() - 1 {
            self.gen.write(", ");
          }
        }
        self.gen.write(")");
      }
      Op::Call(_class, name, args) => {
        self.gen.write(val);
        if !(name == "get" && args.is_empty()) {
          self.gen.write(".");
          if name == "read_str" && args.is_empty() {
            self.gen.write("read_str(32767)");
          } else if name == "read_byte_arr" && args.len() == 1 {
            self.gen.write("read_byte_arr_max(");
            for a in args.iter() {
              self.write_expr(a);
            }
            self.gen.write(")");
          } else if name == "read_map" && args.len() == 3 {
            self.gen.write("read_map(");
            for (idx, a) in args.iter().enumerate().skip(1) {
              self.write_expr(a);
              if idx != args.len() - 1 {
                self.gen.write(", ");
              }
            }
            self.gen.write(")");
          } else if name == "read_collection" && args.len() == 2 {
            let mut args = args.clone();
            match &args[0].initial {
              Value::MethodRef(class, name) if class == "HashSet" && name == "with_capacity" => {
                self.gen.write("read_set(");
              }
              Value::MethodRef(class, name) if class == "Vec" && name == "with_capacity" => {
                self.gen.write("read_list(");
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
                    self.gen.write("read_set_max(");
                    args.push(len);
                  }
                  Value::MethodRef(class, name)
                    if class == "com/google/common/collect/Lists"
                      && name == "new_array_list_with_capacity" =>
                  {
                    self.gen.write("read_list_max(");
                    args.push(len);
                  }
                  _ => panic!("unexpected read_collection args {:?}", inner_args),
                }
              }
              _ => panic!("unexpected read_collection args {:?}", args),
            }
            for (idx, a) in args.iter().enumerate().skip(1) {
              self.write_expr(a);
              if idx != args.len() - 1 {
                self.gen.write(", ");
              }
            }
            self.gen.write(")");
          } else {
            self.gen.write(name);
            self.gen.write("(");
            for (idx, a) in args.iter().enumerate() {
              self.write_expr(a);
              if idx != args.len() - 1 {
                self.gen.write(", ");
              }
            }
            self.gen.write(")");
          }
        }
      }

      Op::Cast(ty) => {
        self.gen.write(val);
        self.gen.write(match ty {
          Type::Byte => " as i8",
          Type::Short => " as i16",
          Type::Int => " as i32",
          Type::Long => " as i64",
          Type::Float => " as f32",
          Type::Double => " as f64",
          _ => unreachable!(),
        });
      }
      Op::As(ty) => {
        self.gen.write(val);
        self.gen.write(" as ");
        self.gen.write(&ty.name);
      }
      Op::Neq(v) => {
        self.gen.write(val);
        self.gen.write(" != ");
        self.write_expr(v);
      }
    }
  }

  fn write_var(&mut self, v: usize) {
    match self.vars.get(v) {
      Some(kind) => match kind {
        VarKind::This => self.gen.write("self"),
        VarKind::Arg => self.gen.write("p"),
        VarKind::Local => {
          self.gen.write("v_");
          self.gen.write(&v.to_string());
        }
      },
      // Probably a special variable, used to signify packet
      None => self.gen.write("p"),
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

      Cond::Bool(val) => {
        self.write_expr(val);
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
}
