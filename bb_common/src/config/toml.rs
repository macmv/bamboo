use indexmap::IndexMap;
use std::fmt;

// This uses very similar techniques to the `toml` crate, but adds support for
// serializing comments from values.

#[derive(Clone, Debug, PartialEq)]
pub struct Value {
  pub comments: Vec<String>,
  pub value:    ValueInner,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ValueInner {
  String(String),
  Integer(i64),
  Float(f64),
  Boolean(bool),
  // Datetime(Datetime),
  Array(Vec<Value>),
  Table(IndexMap<String, Value>),
}

impl Value {
  pub fn is_array(&self) -> bool { matches!(self.value, ValueInner::Array(_)) }
  pub fn is_table(&self) -> bool { matches!(self.value, ValueInner::Table(_)) }
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.value.fmt(f) }
}
impl fmt::Display for ValueInner {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::String(v) => write!(f, "\"{v}\""),
      Self::Integer(v) => write!(f, "{v}"),
      Self::Float(v) => write!(f, "{v}"),
      Self::Boolean(v) => write!(f, "{v}"),
      Self::Array(v) => {
        write!(f, "[{}]", v.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", "))
      }
      Self::Table(v) => write!(
        f,
        "{{{}}}",
        v.iter().map(|(k, v)| format!("{k} = {v}")).collect::<Vec<String>>().join(", ")
      ),
    }
  }
}

pub struct Path {
  pub segments: Vec<String>,
}

impl fmt::Display for Path {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (i, elem) in self.segments.iter().enumerate() {
      if i != 0 {
        write!(f, ".")?;
      }
      write!(f, "{elem}")?;
    }
    Ok(())
  }
}

impl Value {
  fn fmt_long(&self, path: Path, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for line in &self.comments {
      write!(f, "# {line}")?;
    }
    self.value.fmt_long(path, f)
  }
}
impl ValueInner {
  fn fmt_long(&self, path: Path, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::String(v) => write!(f, "\"{v}\""),
      Self::Integer(v) => write!(f, "{v}"),
      Self::Float(v) => write!(f, "{v}"),
      Self::Boolean(v) => write!(f, "{v}"),
      Self::Array(arr) => {
        write!(f, "[{}]", arr.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(", "))
      }
      Self::Table(table) => {
        if table.values().any(|v| !v.comments.is_empty() || v.is_array() || v.is_table()) {
          if !path.segments.is_empty() {
            write!(f, "[{path}]")?;
          }
          for (k, v) in table {
            path.segments.push(k.clone());
            v.fmt_long(path, f)?;
            path.segments.pop();
          }
          Ok(())
        } else {
          write!(
            f,
            "{{{}}}",
            table.iter().map(|(k, v)| format!("{k} = {v}")).collect::<Vec<String>>().join(", ")
          )
        }
      }
    }
  }
}
