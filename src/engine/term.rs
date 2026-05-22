use core::fmt::Display;

use crate::syntax::Atom;

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Literal(Atom),
    App(String, Vec<Term>),
    Cyclic,
}

impl Display for Term {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Term::Literal(atom) => write!(f, "{atom}"),
            Term::App(func, terms) => write!(
                f,
                "({} {})",
                func,
                terms
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            Term::Cyclic => write!(f, "..."),
        }
    }
}
