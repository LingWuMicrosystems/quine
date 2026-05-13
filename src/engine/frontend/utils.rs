#[derive(Debug, Default, Clone, Copy)]
pub struct AnonymousVarCounter(pub usize);

impl AnonymousVarCounter {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

use crate::{engine::interner::Interner, regraph::common::{Atom, Value}};

pub fn atom_to_value(atom: &Atom, interner: &mut Interner) -> Value {
    match atom {
        Atom::Str(s) => Value(interner.intern(s.clone()) as u64),
        other => other.to_value(),
    }
}
