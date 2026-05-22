pub mod body2action;
pub mod head2query;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use quine_core::rule::VariableRecord;
use quine_core::types::*;
use quine_core::{common::*, rule};

use crate::compile::body2action::{CompileCtx, bodys2action};
use crate::compile::head2query::heads2query;
use crate::env::{CompileEnv, TableEnv};
use crate::error::CompileError;
use crate::interner::Interner;
use crate::syntax::{self, Atom, Bodys, Command};
use crate::{CompiledUnit, NativeSignature};

pub struct Compiler;

impl Compiler {
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
                    cols.push(Type::Base(BaseType::Id));
                    let table_def = TableDef(table_name.clone(), cols.into());
                    table_types.insert(table_name, table_def.clone())?;
                    table_defs.push(table_def);
                }
                data_types.insert(name.clone(), type_def.clone())?;
                Ok(CompiledUnit {
                    table_defs,
                    rules: vec![],
                    actions: vec![],
                })
            }
            Command::TableDef(name, table_def) => {
                for ty in table_def.1.iter() {
                    check_type_defined(ty, data_types)?;
                }
                table_types.insert(name.clone(), table_def.clone())?;
                Ok(CompiledUnit {
                    table_defs: vec![table_def.clone()],
                    rules: vec![],
                    actions: vec![],
                })
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
                Ok(CompiledUnit {
                    table_defs: vec![],
                    rules: vec![compiled],
                    actions: vec![],
                })
            }
            Command::Fact(fact) => {
                Self::compile_fact(fact, table_types, interner, native_names, native_signatures)
            }
            Command::Query(..) | Command::Run => Ok(CompiledUnit {
                table_defs: vec![],
                rules: vec![],
                actions: vec![],
            }),
        }
    }

    pub fn compile_fact(
        fact: &Bodys,
        table_types: &TableEnv,
        interner: &mut Interner,
        native_names: &Map<String, usize>,
        native_signatures: &Map<String, NativeSignature>,
    ) -> Result<CompiledUnit, CompileError> {
        let mut ctx = CompileCtx {
            table_map: &table_types.name_map,
            head_variables: &VariableRecord::default(),
            variables: VariableRecord::default(),
            lets: Vec::new(),
            interner,
            native_names,
            native_signatures,
        };
        let action = bodys2action(&mut ctx, fact)?;
        Ok(CompiledUnit {
            table_defs: vec![],
            rules: vec![],
            actions: vec![action],
        })
    }
}

fn check_type_defined(ty: &Type, data_types: &CompileEnv) -> Result<(), CompileError> {
    match ty {
        Type::Name(name) if !data_types.name2type_map.contains_key(name) => {
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
