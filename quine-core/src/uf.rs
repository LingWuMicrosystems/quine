use core::mem::swap;

use alloc::vec::Vec;

use crate::common::Value;

#[derive(Debug, Default, Clone)]
pub struct UnionFind {
    pub parents: Vec<Value>,
}

impl UnionFind {
    pub fn add(&mut self, id: Value) {
        self.parents.push(id);
    }

    /// non compressed find
    pub fn find(&self, id: Value) -> Value {
        let mut cur = id;
        loop {
            let parent = self.parents[cur.0 as usize];
            if cur == parent {
                return cur;
            }
            cur = parent;
        }
    }

    /// compressed find
    #[inline]
    pub fn find_compress(&mut self, id: Value) -> Value {
        let mut cur = id;
        loop {
            let parent = self.parents[cur.0 as usize];
            if cur == parent {
                return cur;
            }
            let grandparent = self.parents[parent.0 as usize];
            self.parents[cur.0 as usize] = grandparent;
            cur = grandparent;
        }
    }

    #[inline]
    pub fn union(&mut self, lhs: Value, rhs: Value) -> Option<(Value, Value)> {
        // loop {
        let mut lhs = self.find_compress(lhs);
        let mut rhs = self.find_compress(rhs);
        if lhs == rhs {
            return None;
        }
        // union by min
        if lhs > rhs {
            swap(&mut lhs, &mut rhs);
        }
        self.parents[rhs.0 as usize] = lhs;
        Some((lhs, rhs))
        // }
    }
}
