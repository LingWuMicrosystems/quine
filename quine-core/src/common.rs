use core::fmt::Display;

use alloc::string::String;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::types::BaseType;

const SIGN_BIT: u64 = 0x8000_0000_0000_0000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Value(pub u64);

impl Value {
    pub fn encode_i8(v: i8) -> Value {
        Value(((v as i64) as u64) ^ SIGN_BIT)
    }
    pub fn encode_i16(v: i16) -> Value {
        Value(((v as i64) as u64) ^ SIGN_BIT)
    }
    pub fn encode_i32(v: i32) -> Value {
        Value(((v as i64) as u64) ^ SIGN_BIT)
    }
    pub fn encode_i64(v: i64) -> Value {
        Value((v as u64) ^ SIGN_BIT)
    }
    pub fn encode_f32(v: f32) -> Value {
        let bits = v.to_bits();
        if bits & 0x8000_0000 != 0 {
            Value((!bits) as u64)
        } else {
            Value((bits ^ 0x8000_0000) as u64)
        }
    }
    pub fn encode_f64(v: f64) -> Value {
        let bits = v.to_bits();
        if bits & SIGN_BIT != 0 {
            Value(!bits)
        } else {
            Value(bits ^ SIGN_BIT)
        }
    }

    pub fn decode_i8(&self) -> i8 {
        ((self.0 ^ SIGN_BIT) as i64) as i8
    }
    pub fn decode_i16(&self) -> i16 {
        ((self.0 ^ SIGN_BIT) as i64) as i16
    }
    pub fn decode_i32(&self) -> i32 {
        ((self.0 ^ SIGN_BIT) as i64) as i32
    }
    pub fn decode_i64(&self) -> i64 {
        (self.0 ^ SIGN_BIT) as i64
    }
    pub fn decode_f32(&self) -> f32 {
        let encoded = self.0 as u32;
        let bits = if encoded & 0x8000_0000 == 0 {
            !encoded
        } else {
            encoded ^ 0x8000_0000
        };
        f32::from_bits(bits)
    }
    pub fn decode_f64(&self) -> f64 {
        let bits = if self.0 & SIGN_BIT == 0 {
            !self.0
        } else {
            self.0 ^ SIGN_BIT
        };
        f64::from_bits(bits)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowIndex(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnIndex(pub usize);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(pub usize);

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct VarName(pub Name);

pub type Map<K, V> = FxHashMap<K, V>;
pub type Set<K> = FxHashSet<K>;

// FIXME: move it
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atom {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    U8(u8),
    U16(u16),
    U32(u32),

    I64(i64),
    U64(u64),
    F32(u32), // IEEE 754 bits
    F64(u64), // IEEE 754 bits
    Str(String),
}

impl Display for Atom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Atom::Bool(b) => write!(f, "{b}"),
            Atom::I8(i) => write!(f, "{i}"),
            Atom::I16(i) => write!(f, "{i}"),
            Atom::I32(i) => write!(f, "{i}"),
            Atom::U8(u) => write!(f, "{u}"),
            Atom::U16(u) => write!(f, "{u}"),
            Atom::U32(u) => write!(f, "{u}"),
            Atom::I64(i) => write!(f, "{i}"),
            Atom::U64(u) => write!(f, "{u}"),
            Atom::F32(bits) => write!(f, "{}", f32::from_bits(*bits)),
            Atom::F64(bits) => write!(f, "{}", f64::from_bits(*bits)),
            Atom::Str(s) => write!(f, "{s}"),
        }
    }
}

impl Atom {
    pub fn get_type(&self) -> BaseType {
        match self {
            Atom::Bool(_) => BaseType::I1,
            Atom::I8(_) => BaseType::I8,
            Atom::I16(_) => BaseType::I16,
            Atom::I32(_) => BaseType::I32,
            Atom::U8(_) => BaseType::U8,
            Atom::U16(_) => BaseType::U16,
            Atom::U32(_) => BaseType::U32,
            Atom::I64(_) => BaseType::I64,
            Atom::U64(_) => BaseType::U64,
            Atom::F32(_) => BaseType::F32,
            Atom::F64(_) => BaseType::F64,
            Atom::Str(_) => BaseType::Str,
        }
    }

    pub fn to_value(&self) -> Value {
        match *self {
            Atom::I8(i) => Value::encode_i8(i),
            Atom::I16(i) => Value::encode_i16(i),
            Atom::I32(i) => Value::encode_i32(i),
            Atom::I64(i) => Value::encode_i64(i),
            Atom::U8(u) => Value(u as u64),
            Atom::U16(u) => Value(u as u64),
            Atom::U32(u) => Value(u as u64),

            Atom::U64(u) => Value(u),
            Atom::Bool(b) => Value(if b { 1u64 } else { 0u64 }),
            Atom::F32(bits) => Value::encode_f32(f32::from_bits(bits)),
            Atom::F64(bits) => Value::encode_f64(f64::from_bits(bits)),
            Atom::Str(_) => unimplemented!("use intern via engine::frontend::utils::atom_to_value"),
        }
    }
}
