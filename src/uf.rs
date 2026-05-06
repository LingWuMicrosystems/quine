use core::mem::swap;

use alloc::vec::Vec;

use crate::common::Id;

#[derive(Debug, Default, Clone)]
pub struct UnionFind {
    pub parents: Vec<Id>,
}

impl UnionFind {
    /// non compressed find
    pub fn find(&self, id: Id) -> Id {
        let mut cur = id;
        loop {
            let parent = self.parents[cur.0 as usize];
            if cur == parent {
                return cur;
            }
            cur = parent;
        }
    }

    /// non compressed find
    #[inline]
    pub fn find_compress(&mut self, id: Id) -> Id {
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
    pub fn union(&mut self, lhs: Id, rhs: Id) -> Option<(Id, Id)> {
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
