use super::{convert, Cond, Expr, Field, Instr, Op, Packet, Type, Value};
use convert_case::{Case, Casing};

pub fn pass(p: &mut Packet) {
  let len = simplify_instr(&mut p.reader.block);
  if let Some(l) = len {
    p.reader.block = p.reader.block[..l].to_vec();
    p.fields.push(Field {
      name:        "unknown".into(),
      ty:          Type::Array(Box::new(Type::Byte)),
      reader_type: None,
      option:      false,
      initialized: false,
    });
  }
  for f in &mut p.fields {
    simplify_name(&mut f.name);
    let (initialized, option) = check_option(&p.reader.block, &f.name);
    if option && matches!(f.ty, Type::Array(_)) {
      f.option = false;
      f.initialized = false;
    } else {
      f.initialized = initialized;
      f.option = option;
    }
  }
}
fn simplify_instr(instr: &mut [Instr]) -> Option<usize> {
  for (idx, i) in instr.iter_mut().enumerate() {
    match i {
      Instr::Super => {}
      Instr::Set(name, v) => {
        simplify_name(name);
        if let Some(new_instr) = simplify_expr_overwrite(v) {
          *i = new_instr;
        } else {
          *i = Instr::Set(
            "unknown".into(),
            Expr::new(Value::Var(1)).op(Op::Call("tcp::Packet".into(), "read_all".into(), vec![])),
          );
          return Some(idx + 1);
        }
      }
      Instr::SetArr(arr, idx, val) => {
        simplify_expr(arr);
        simplify_val(idx);
        simplify_expr(val);
      }
      Instr::Let(_, val) => simplify_expr(val),
      Instr::Expr(v) => match v.initial {
        Value::Var(0) => match v.ops.first_mut().unwrap() {
          Op::Call(_, name, args) => {
            let instr = convert::this_call(name, args);
            let mut arr = vec![instr];
            let res = simplify_instr(&mut arr);
            *i = arr.pop().unwrap();
            if res.is_some() {
              return Some(idx + 1);
            }
          }
          v => panic!("unknown op on self: {:?}", v),
        },
        _ => simplify_expr(v),
      },
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
      Instr::Return(v) => simplify_expr(v),
    }
  }
  None
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
  expr.ops.iter_mut().for_each(|op| simplify_op(op));
  convert::overwrite(expr);
}
fn simplify_expr_overwrite(expr: &mut Expr) -> Option<Instr> {
  simplify_val(&mut expr.initial);
  expr.ops.iter_mut().for_each(|op| simplify_op(op));
  convert::overwrite(expr)
}
fn simplify_val(val: &mut Value) {
  match val {
    Value::Field(name) => simplify_name(name),
    Value::Static(..) => {}
    Value::Array(len) => simplify_expr(len),
    Value::CallStatic(class, name, args) => {
      simplify_name(name);
      *class = convert::class(class);
      let (new_class, new_name) = convert::static_call(&class, &name);
      *class = new_class.into();
      *name = new_name.into();
      args.iter_mut().for_each(|a| simplify_expr(a))
    }
    Value::MethodRef(class, name) => {
      simplify_name(name);
      *class = convert::class(class);
      *val = convert::static_ref(&class, &name);
    }
    Value::Closure(args, block) => {
      for a in args.iter_mut() {
        simplify_expr(a);
      }
      simplify_instr(&mut block.block);
    }
    Value::New(_, args) => {
      args.iter_mut().for_each(|a| simplify_expr(a));
    }
    Value::Null | Value::Lit(_) | Value::Var(_) => {}
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
    Op::Call(class, name, args) => {
      *class = convert::class(class);
      simplify_name(name);
      let (new_name, new_args) = convert::member_call(class, name);
      *name = new_name.into();
      if let Some(a) = new_args {
        *args = a;
      } else {
        args.iter_mut().for_each(|a| simplify_expr(a))
      }
    }
    Op::Cast(_) => {}
  }
}
fn simplify_name(name: &mut String) {
  if name == "type" {
    *name = "ty".into();
  } else {
    *name = name.to_case(Case::Snake);
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
