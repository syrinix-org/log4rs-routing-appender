use serde_value::Value;
use ordered_float::OrderedFloat;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fmt::Write;
use log_mdc;

use route::pattern::parser::{Parser, Piece};

pub struct Template {
    value: ValueTemplate,
    keys: HashSet<String>,
}

impl Template {
    pub fn new(pattern: &Value) -> Result<Template, Box<dyn Error + Sync + Send>> {
        let value = ValueTemplate::new(pattern)?;
        let mut keys = HashSet::new();
        value.keys(&mut keys);
        Ok(Template {
            value: value,
            keys: keys,
        })
    }

    pub fn key(&self) -> String {
        let mut s = String::new();
        for key in &self.keys {
            log_mdc::get(key, |k| match k {
                Some(k) => write!(s, "{}{}", k.len(), k).unwrap(),
                None => s.push('-'),
            });
        }
        s
    }

    pub fn expand(&self) -> Result<Value, Box<dyn Error + Sync + Send>> {
        self.value.expand()
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq)]
enum Chunk {
    Text(String),
    Mdc {
        key: String,
        default: Option<String>,
    },
}

enum ValueTemplate {
    Map(BTreeMap<ValueTemplate, ValueTemplate>),
    Newtype(Box<ValueTemplate>),
    Option(Option<Box<ValueTemplate>>),
    Seq(Vec<ValueTemplate>),
    String(Vec<Chunk>),
    Bool(bool),
    Bytes(Vec<u8>),
    Char(char),
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Unit,
}

impl Eq for ValueTemplate {}

impl PartialEq for ValueTemplate {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&ValueTemplate::Bool(v0), &ValueTemplate::Bool(v1)) if v0 == v1 => true,
            (&ValueTemplate::U8(v0), &ValueTemplate::U8(v1)) if v0 == v1 => true,
            (&ValueTemplate::U16(v0), &ValueTemplate::U16(v1)) if v0 == v1 => true,
            (&ValueTemplate::U32(v0), &ValueTemplate::U32(v1)) if v0 == v1 => true,
            (&ValueTemplate::U64(v0), &ValueTemplate::U64(v1)) if v0 == v1 => true,
            (&ValueTemplate::I8(v0), &ValueTemplate::I8(v1)) if v0 == v1 => true,
            (&ValueTemplate::I16(v0), &ValueTemplate::I16(v1)) if v0 == v1 => true,
            (&ValueTemplate::I32(v0), &ValueTemplate::I32(v1)) if v0 == v1 => true,
            (&ValueTemplate::I64(v0), &ValueTemplate::I64(v1)) if v0 == v1 => true,
            (&ValueTemplate::F32(v0), &ValueTemplate::F32(v1)) if OrderedFloat(v0) ==
                                                                  OrderedFloat(v1) => true,
            (&ValueTemplate::F64(v0), &ValueTemplate::F64(v1)) if OrderedFloat(v0) ==
                                                                  OrderedFloat(v1) => true,
            (&ValueTemplate::Char(v0), &ValueTemplate::Char(v1)) if v0 == v1 => true,
            (&ValueTemplate::String(ref v0), &ValueTemplate::String(ref v1)) if v0 == v1 => true,
            (&ValueTemplate::Unit, &ValueTemplate::Unit) => true,
            (&ValueTemplate::Option(ref v0), &ValueTemplate::Option(ref v1)) if v0 == v1 => true,
            (&ValueTemplate::Newtype(ref v0), &ValueTemplate::Newtype(ref v1)) if v0 == v1 => true,
            (&ValueTemplate::Seq(ref v0), &ValueTemplate::Seq(ref v1)) if v0 == v1 => true,
            (&ValueTemplate::Map(ref v0), &ValueTemplate::Map(ref v1)) if v0 == v1 => true,
            (&ValueTemplate::Bytes(ref v0), &ValueTemplate::Bytes(ref v1)) if v0 == v1 => true,
            _ => false,
        }
    }
}

impl PartialOrd for ValueTemplate {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for ValueTemplate {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match (self, rhs) {
            (&ValueTemplate::Bool(v0), &ValueTemplate::Bool(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::U8(v0), &ValueTemplate::U8(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::U16(v0), &ValueTemplate::U16(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::U32(v0), &ValueTemplate::U32(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::U64(v0), &ValueTemplate::U64(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::I8(v0), &ValueTemplate::I8(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::I16(v0), &ValueTemplate::I16(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::I32(v0), &ValueTemplate::I32(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::I64(v0), &ValueTemplate::I64(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::F32(v0), &ValueTemplate::F32(v1)) => {
                OrderedFloat(v0).cmp(&OrderedFloat(v1))
            }
            (&ValueTemplate::F64(v0), &ValueTemplate::F64(v1)) => {
                OrderedFloat(v0).cmp(&OrderedFloat(v1))
            }
            (&ValueTemplate::Char(v0), &ValueTemplate::Char(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::String(ref v0), &ValueTemplate::String(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::Unit, &ValueTemplate::Unit) => Ordering::Equal,
            (&ValueTemplate::Option(ref v0), &ValueTemplate::Option(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::Newtype(ref v0), &ValueTemplate::Newtype(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::Seq(ref v0), &ValueTemplate::Seq(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::Map(ref v0), &ValueTemplate::Map(ref v1)) => v0.cmp(v1),
            (&ValueTemplate::Bytes(ref v0), &ValueTemplate::Bytes(ref v1)) => v0.cmp(v1),
            (ref v0, ref v1) => v0.discriminant().cmp(&v1.discriminant()),
        }
    }
}

impl ValueTemplate {
    fn new(value: &Value) -> Result<ValueTemplate, Box<dyn Error + Sync + Send>> {
        let value = match *value {
            Value::Map(ref m) => {
                let mut m2 = BTreeMap::new();
                for (k, v) in m {
                    m2.insert(ValueTemplate::new(k)?, ValueTemplate::new(v)?);
                }
                ValueTemplate::Map(m2)
            }
            Value::Newtype(ref v) => ValueTemplate::Newtype(Box::new(ValueTemplate::new(v)?)),
            Value::Option(ref v) => {
                let v = match *v {
                    Some(ref v) => Some(Box::new(ValueTemplate::new(v)?)),
                    None => None,
                };
                ValueTemplate::Option(v)
            }
            Value::Seq(ref vs) => {
                let mut vs2 = vec![];
                for v in vs {
                    vs2.push(ValueTemplate::new(v)?);
                }
                ValueTemplate::Seq(vs2)
            }
            Value::String(ref s) => {
                let mut chunks = vec![];
                for piece in Parser::new(s) {
                    let c = match piece {
                        Piece::Text(t) => Chunk::Text(t.to_owned()),
                        Piece::Argument { name: "mdc", args } => {
                            if args.is_empty() || args.len() > 2 {
                                return Err(format!("expected 1 or 2 arguments: `{}`", s).into());
                            }
                            Chunk::Mdc {
                                key: args[0].to_owned(),
                                default: args.get(1).map(|&s| s.to_owned()),
                            }
                        }
                        Piece::Argument { name, .. } => {
                            return Err(format!("unknown argument `{}`: `{}`", name, s).into());
                        }
                        Piece::Error(e) => return Err(format!("{}: `{}`", e, s).into()),
                    };
                    chunks.push(c);
                }
                ValueTemplate::String(chunks)
            }
            Value::Bool(b) => ValueTemplate::Bool(b),
            Value::Bytes(ref b) => ValueTemplate::Bytes(b.clone()),
            Value::Char(c) => ValueTemplate::Char(c),
            Value::F32(f) => ValueTemplate::F32(f),
            Value::F64(f) => ValueTemplate::F64(f),
            Value::I8(i) => ValueTemplate::I8(i),
            Value::I16(i) => ValueTemplate::I16(i),
            Value::I32(i) => ValueTemplate::I32(i),
            Value::I64(i) => ValueTemplate::I64(i),
            Value::U8(u) => ValueTemplate::U8(u),
            Value::U16(u) => ValueTemplate::U16(u),
            Value::U32(u) => ValueTemplate::U32(u),
            Value::U64(u) => ValueTemplate::U64(u),
            Value::Unit => ValueTemplate::Unit,
        };
        Ok(value)
    }

    fn discriminant(&self) -> usize {
        match *self {
            ValueTemplate::Bool(..) => 0,
            ValueTemplate::U8(..) => 1,
            ValueTemplate::U16(..) => 2,
            ValueTemplate::U32(..) => 3,
            ValueTemplate::U64(..) => 4,
            ValueTemplate::I8(..) => 5,
            ValueTemplate::I16(..) => 6,
            ValueTemplate::I32(..) => 7,
            ValueTemplate::I64(..) => 8,
            ValueTemplate::F32(..) => 9,
            ValueTemplate::F64(..) => 10,
            ValueTemplate::Char(..) => 11,
            ValueTemplate::String(..) => 12,
            ValueTemplate::Unit => 13,
            ValueTemplate::Option(..) => 14,
            ValueTemplate::Newtype(..) => 15,
            ValueTemplate::Seq(..) => 16,
            ValueTemplate::Map(..) => 17,
            ValueTemplate::Bytes(..) => 18,
        }
    }

    fn keys(&self, keys: &mut HashSet<String>) {
        match *self {
            ValueTemplate::Map(ref m) => {
                for (k, v) in m {
                    k.keys(keys);
                    v.keys(keys);
                }
            }
            ValueTemplate::Newtype(ref v) => v.keys(keys),
            ValueTemplate::Option(ref v) => {
                if let Some(ref v) = *v {
                    v.keys(keys);
                }
            }
            ValueTemplate::Seq(ref vs) => {
                for v in vs {
                    v.keys(keys);
                }
            }
            ValueTemplate::String(ref chunks) => {
                for chunk in chunks {
                    if let Chunk::Mdc { ref key, .. } = *chunk {
                        keys.insert(key.clone());
                    }
                }
            }
            _ => {}
        }
    }

    fn expand(&self) -> Result<Value, Box<dyn Error + Sync + Send>> {
        let v = match *self {
            ValueTemplate::Map(ref m) => {
                let mut m2 = BTreeMap::new();
                for (k, v) in m {
                    m2.insert(k.expand()?, v.expand()?);
                }
                Value::Map(m2)
            }
            ValueTemplate::Newtype(ref v) => Value::Newtype(Box::new(v.expand()?)),
            ValueTemplate::Option(ref v) => {
                match *v {
                    Some(ref v) => Value::Option(Some(Box::new(v.expand()?))),
                    None => Value::Option(None),
                }
            }
            ValueTemplate::Seq(ref vs) => {
                let mut vs2 = Vec::with_capacity(vs.len());
                for v in vs {
                    vs2.push(v.expand()?);
                }
                Value::Seq(vs2)
            }
            ValueTemplate::String(ref chunks) => {
                let mut s = String::new();
                for chunk in chunks {
                    match *chunk {
                        Chunk::Text(ref t) => s.push_str(t),
                        Chunk::Mdc { ref key, ref default } => {
                            log_mdc::get(key, |v| match (v, default.as_ref().map(|s| &**s)) {
                                (Some(v), _) | (None, Some(v)) => {
                                    s.push_str(v);
                                    Ok(())
                                }
                                (None, None) => Err(format!("MDC key `{}` not present", key)),
                            })?
                        }
                    }
                }
                Value::String(s)
            }
            ValueTemplate::Bool(b) => Value::Bool(b),
            ValueTemplate::Bytes(ref b) => Value::Bytes(b.clone()),
            ValueTemplate::Char(c) => Value::Char(c),
            ValueTemplate::F32(f) => Value::F32(f),
            ValueTemplate::F64(f) => Value::F64(f),
            ValueTemplate::I8(i) => Value::I8(i),
            ValueTemplate::I16(i) => Value::I16(i),
            ValueTemplate::I32(i) => Value::I32(i),
            ValueTemplate::I64(i) => Value::I64(i),
            ValueTemplate::U8(i) => Value::U8(i),
            ValueTemplate::U16(i) => Value::U16(i),
            ValueTemplate::U32(i) => Value::U32(i),
            ValueTemplate::U64(i) => Value::U64(i),
            ValueTemplate::Unit => Value::Unit,
        };

        Ok(v)
    }
}
