pub mod body2action;
pub mod head2query;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use quine_core::rule::{Action, VariableRecord};
use quine_core::types::*;
use quine_core::{common::*, rule};

use crate::compile::body2action::{CompileCtx, bodys2action};
use crate::compile::head2query::heads2query;
use crate::env::{CompileEnv, TableEnv};
use crate::error::CompileError;
use crate::interner::Interner;
use crate::syntax::{self, Atom, AtomOrVariable, Bodys, Command, Expr, VarExtractor};
use crate::{CompiledUnit, NativeSignature};

pub fn unify(t1: &Type, t2: &Type) -> Result<(), CompileError> {
    if t1 == t2 {
        Ok(())
    } else {
        Err(CompileError::TypeCheckError(t1.clone(), t2.clone()))
    }
}

pub fn compile_command(
    cmd: &Command,
    data_types: &mut CompileEnv,
    table_types: &mut TableEnv,
    interner: &mut Interner,
    native_names: &Map<String, usize>,
    native_signatures: &Map<String, NativeSignature>,
) -> Result<CompiledUnit, CompileError> {
    match cmd {
        Command::TypeDef(name, type_def) => {
            let mut table_defs = vec![];
            for cons in &type_def.1.0 {
                let table_name = format!("{}.{}", name, cons.0);
                let mut cols: Vec<Type> = cons.1.to_vec();
                cols.push(Type::Name(name.clone()));
                let table_def = TableDef(table_name.clone(), cols.into(), None);
                table_types.insert(table_name, table_def.clone())?;
                table_defs.push(table_def);
            }
            data_types.insert(name.clone(), type_def.clone())?;
            Ok(CompiledUnit::TableDefs(table_defs.into()))
        }
        Command::TableDef(name, table_def) => {
            for ty in table_def.1.iter() {
                check_type_defined(ty, data_types)?;
            }
            let result_ty = &table_def.1[table_def.1.len() - 1];
            let needs_merge = !matches!(result_ty, Type::Name(_) | Type::Base(BaseType::Id));
            match (&table_def.2, needs_merge) {
                (Some(_), true) => match result_ty {
                    Type::Base(bt) if bt.is_numeric() => {}
                    _ => return Err(CompileError::MergeOnNonNumeric(name.clone())),
                },
                (None, true) => return Err(CompileError::MergeRequired(name.clone())),
                _ => {}
            }
            table_types.insert(name.clone(), table_def.clone())?;
            Ok(CompiledUnit::TableDefs([table_def.clone()].into()))
        }
        Command::Rule(rule) => {
            let compiled = compile_rule(
                rule,
                table_types,
                data_types,
                interner,
                native_names,
                native_signatures,
            )?;
            Ok(CompiledUnit::Rule(rule.group.clone(), compiled))
        }
        Command::Fact(fact) => compile_fact(
            fact,
            table_types,
            data_types,
            interner,
            native_names,
            native_signatures,
        )
        .map(CompiledUnit::Action),
        Command::Load(_) => unreachable!("Load handled before compilation"),
        Command::Query(_, _) => unreachable!("Query compiled separately"),
        Command::CostDef(def) => {
            // Validate: type_name must be a defined data type
            if !data_types.name2type_map.contains_key(&def.type_name) {
                return Err(CompileError::UnknownTypeName(def.type_name.clone()));
            }
            // Validate: constructor must exist on the type
            // cons2type_map keys are "TypeName.ConstructorName"
            let full_name = format!("{}.{}", def.type_name, def.constructor);
            if !data_types.cons2type_map.contains_key(&full_name) {
                return Err(CompileError::UnknownConstructor(
                    def.type_name.clone(),
                    def.constructor.clone(),
                ));
            }
            Ok(CompiledUnit::CostDef(def.clone()))
        }
        Command::Extract(expr, mode) => {
            // Validate: no variables allowed in extract expressions
            let vars = expr.extract_vars();
            if let Some(var) = vars.iter().next() {
                return Err(CompileError::VariableInExtract(var.clone()));
            }
            // Validate: constructors referenced in the expression exist
            validate_extract_expr(expr, table_types, data_types)?;
            Ok(CompiledUnit::Extract(expr.clone(), mode.clone()))
        }
        Command::Run(run) => Ok(CompiledUnit::Run(run.clone())),
    }
}

pub fn compile_fact(
    fact: &Bodys,
    table_types: &TableEnv,
    data_types: &CompileEnv,
    interner: &mut Interner,
    native_names: &Map<String, usize>,
    native_signatures: &Map<String, NativeSignature>,
) -> Result<Action, CompileError> {
    let mut ctx = CompileCtx {
        table_map: &table_types.name_map,
        table_defs: &table_types.tables,
        data_types,
        head_variables: &VariableRecord::default(),
        variables: VariableRecord::default(),
        lets: Vec::new(),
        interner,
        native_names,
        native_signatures,
    };
    bodys2action(&mut ctx, fact)
}

fn check_type_defined(ty: &Type, data_types: &CompileEnv) -> Result<(), CompileError> {
    match ty {
        Type::Name(name)
            if !data_types.name2type_map.contains_key(name)
                && !data_types.pending_names.contains(name) =>
        {
            Err(CompileError::UnknownTypeName(name.clone()))
        }
        _ => Ok(()),
    }
}

fn compile_rule(
    rule: &syntax::Rule,
    table_types: &TableEnv,
    data_types: &CompileEnv,
    interner: &mut Interner,
    native_names: &Map<String, usize>,
    native_signatures: &Map<String, NativeSignature>,
) -> Result<rule::Rule, CompileError> {
    let query = heads2query(&rule.heads, table_types, data_types, interner)?;
    let mut ctx = CompileCtx {
        table_map: &table_types.name_map,
        table_defs: &table_types.tables,
        data_types,
        head_variables: &query.variables,
        variables: VariableRecord::default(),
        lets: Vec::new(),
        interner,
        native_names,
        native_signatures,
    };
    let action = bodys2action(&mut ctx, &rule.bodys)?;
    Ok(rule::Rule { query, action })
}

pub fn atom_to_value(atom: Atom, interner: &mut Interner) -> Value {
    match atom {
        Atom::Str(s) => Value(interner.intern(s.clone()) as u64),
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
    }
}

/// Recursively validate that all constructor names in an extract expression
/// exist in table_types or data_types.
fn validate_extract_expr(
    expr: &Expr,
    table_types: &TableEnv,
    data_types: &CompileEnv,
) -> Result<(), CompileError> {
    match expr {
        Expr::AtomOrVariable(AtomOrVariable::Variable(name)) => {
            Err(CompileError::VariableInExtract(name.clone()))
        }
        Expr::AtomOrVariable(AtomOrVariable::Atom(_)) => Ok(()),
        Expr::FunctionCall(call) => {
            // Try direct table lookup first
            if table_types.name_map.contains_key(&call.0) {
                // Direct table name match — valid
            } else if data_types.cons2type_map.keys().any(|k| k.ends_with(&format!(".{}", call.0))) {
                // Constructor name found in cons2type_map — valid
            } else {
                return Err(CompileError::UnknownConstructor(
                    call.0.clone(),
                    call.0.clone(),
                ));
            }
            // Recursively validate arguments
            for arg in call.1.iter() {
                validate_extract_expr(arg, table_types, data_types)?;
            }
            Ok(())
        }
    }
}
