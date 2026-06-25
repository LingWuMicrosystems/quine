use quine_core::common::Value;
use quine_core::related_egraph::RelatedEGraph;
use quine_core::types::{BaseType, Type};
use quine_core::atom::Atom;

/// Returns true if the type represents an eclass reference (not a literal).
///
/// Eclass-typed columns are `Type::Name(_)` or `Type::Base(BaseType::Id)`.
/// This matches the check in `materialize_cheapest_inner`.
pub fn type_is_eclass(ty: &Type) -> bool {
    matches!(ty, Type::Name(_) | Type::Base(BaseType::Id))
}

/// Look up the constructor cost for a table by name.
///
/// Returns the cost from `regraph.cost_models` if present, 0 otherwise.
/// This provides the per-constructor cost used in the ILP objective function.
pub fn constructor_cost(regraph: &RelatedEGraph, table_name: &str) -> u64 {
    regraph.get_constructor_cost(table_name)
}

/// Convert a `Value` + `Type` into an `Atom` for use in `Term::Literal`.
///
/// For `Type::Base(base_ty)`: decodes the Value according to its base type
/// (e.g., `Value` → `Atom::I64` for `BaseType::I64`).
///
/// For `Type::Name(_)`: produces `Atom::U64(val.0)` as a raw representation,
/// since named types represent eclass references, not literal values.
///
/// Note: `BaseType::Str` decoding requires an interner, which is not available
/// in the solver crate. Str values are emitted as `Atom::U64(val.0)`.
pub fn atom_from_value(val: Value, ty: &Type) -> Atom {
    match ty {
        Type::Name(_) => Atom::U64(val.0),
        Type::Base(base) => match base {
            BaseType::Id => Atom::U64(val.0),
            BaseType::I1 => Atom::Bool(val.0 != 0),
            BaseType::I8 => Atom::I8(val.decode_i8()),
            BaseType::U8 => Atom::U8(val.0 as u8),
            BaseType::I16 => Atom::I16(val.decode_i16()),
            BaseType::U16 => Atom::U16(val.0 as u16),
            BaseType::I32 => Atom::I32(val.decode_i32()),
            BaseType::U32 => Atom::U32(val.0 as u32),
            BaseType::I64 => Atom::I64(val.decode_i64()),
            BaseType::U64 => Atom::U64(val.0),
            BaseType::F32 => Atom::F32(val.decode_f32().to_bits()),
            BaseType::F64 => Atom::F64(val.decode_f64().to_bits()),
            // Str decoding requires an interner; emit raw Value as fallback
            BaseType::Str => Atom::U64(val.0),
        },
    }
}
