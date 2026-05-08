use crate::common::Name;

pub enum TypeCheckError {
    DuplicateName(Name),
}
