use rustc_hash::{FxHashMap, FxHashSet};

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
