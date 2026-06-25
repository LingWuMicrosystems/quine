use core::fmt::Display;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

use crate::atom::Atom;

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Literal(Atom),
    App(String, Vec<Term>),
    Let(Vec<(String, Term)>, Box<Term>),
    Var(String),
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
            Term::Let(bindings, body) => {
                write!(f, "(let (")?;
                for (name, val) in bindings {
                    write!(f, "[{name} {val}]")?;
                }
                write!(f, ") {body})")
            }
            Term::Var(name) => write!(f, "{name}"),
            Term::Cyclic => write!(f, "..."),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::vec;

    use super::*;
    use crate::atom::Atom;

    #[test]
    fn test_display_let_single_binding() {
        let term = Term::Let(
            vec![("_t0".into(), Term::App("+".into(), vec![
                Term::Literal(Atom::I32(1)),
                Term::Literal(Atom::I32(2)),
            ]))],
            Box::new(Term::App("f".into(), vec![
                Term::Var("_t0".into()),
                Term::Var("_t0".into()),
            ])),
        );
        let output = format!("{term}");
        assert_eq!(output, "(let ([_t0 (+ 1 2)]) (f _t0 _t0))");
    }

    #[test]
    fn test_display_let_multiple_bindings() {
        let term = Term::Let(
            vec![
                ("_t0".into(), Term::App("+".into(), vec![
                    Term::Literal(Atom::I32(1)),
                    Term::Literal(Atom::I32(2)),
                ])),
                ("_t1".into(), Term::App("*".into(), vec![
                    Term::Literal(Atom::I32(3)),
                    Term::Literal(Atom::I32(4)),
                ])),
            ],
            Box::new(Term::App("f".into(), vec![
                Term::Var("_t0".into()),
                Term::Var("_t1".into()),
            ])),
        );
        let output = format!("{term}");
        assert_eq!(
            output,
            "(let ([_t0 (+ 1 2)][_t1 (* 3 4)]) (f _t0 _t1))"
        );
    }

    #[test]
    fn test_display_let_no_bindings_not_used() {
        // When there are no shared sub-expressions, no Let should be created.
        // This test verifies the regular App display is unchanged.
        let term = Term::App("f".into(), vec![
            Term::Literal(Atom::I32(42)),
        ]);
        let output = format!("{term}");
        assert_eq!(output, "(f 42)");
    }

    #[test]
    fn test_display_var() {
        let term = Term::Var("_t0".into());
        let output = format!("{term}");
        assert_eq!(output, "_t0");
    }

    #[test]
    fn test_display_cyclic_unchanged() {
        let term = Term::Cyclic;
        let output = format!("{term}");
        assert_eq!(output, "...");
    }
}
