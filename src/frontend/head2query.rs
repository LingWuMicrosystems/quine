use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{
    common::{ColumnIndex, Map, Set, VarId},
    frontend::{
        env::TableEnv,
        error::CompileError,
        syntax::{AtomOrVariable, ConstructorPattern, Head, Pattern},
    },
    regraph::rule::{self, Constraint, CrossConstraint, FusedScan, Query, VariableRecord},
    types::Type,
};

pub fn heads2query(heads: &[Head], table_env: &TableEnv) -> Result<Query, CompileError> {
    let mut variables = VariableRecord::default();

    let mut scans: Map<VarId, Vec<FusedScan>> = Map::default();
    let mut guard_cmps: Map<VarId, Set<Constraint>> = Map::default();
    let mut constraints: Set<CrossConstraint> = Set::default();

    for head in heads {
        match head {
            Head::Match(constructor_pattern) => {
                check_and_compile_con_pattern(
                    constructor_pattern,
                    table_env,
                    false,
                    &mut variables,
                    &mut scans,
                    &mut guard_cmps,
                    &mut constraints,
                )?;
            }
            Head::Guard(op, var, AtomOrVariable::Atom(a)) => {
                let Some(var) = variables.get_offset(var) else {
                    return Err(CompileError::VariableNotDefine(var.clone()));
                };
                let defined_type = variables.get_type(var).unwrap();
                let atom_ty = a.get_type();
                let Type::Base(def_type) = defined_type else {
                    return Err(CompileError::TypeCheckError(
                        Type::Base(atom_ty),
                        defined_type.clone(),
                    ));
                };
                if def_type != &atom_ty {
                    return Err(CompileError::TypeCheckError(
                        Type::Base(atom_ty),
                        defined_type.clone(),
                    ));
                }
                guard_cmps
                    .entry(VarId(var))
                    .or_default()
                    .insert(Constraint {
                        op: op.to_constraint_op(atom_ty.is_sign()),
                        value: a.clone().to_value(),
                    });
            }
            Head::Guard(op, lhs, AtomOrVariable::Variable(rhs)) => {
                let Some(lhs_offset) = variables.get_offset(lhs) else {
                    return Err(CompileError::VariableNotDefine(lhs.clone()));
                };
                let Some(rhs_offset) = variables.get_offset(rhs) else {
                    return Err(CompileError::VariableNotDefine(rhs.clone()));
                };
                let lhs_ty = variables.get_type(lhs_offset).unwrap();
                let rhs_ty = variables.get_type(rhs_offset).unwrap();
                if lhs != rhs {
                    return Err(CompileError::TypeCheckError(rhs_ty.clone(), lhs_ty.clone()));
                }
                constraints.insert(CrossConstraint {
                    op: op.to_constraint_op(lhs_ty.is_sign()),
                    lhs: VarId(lhs_offset),
                    rhs: VarId(rhs_offset),
                });
            }
            Head::LetEq(Pattern::Variable(lhs), Pattern::Variable(rhs)) => {
                let Some(lhs_offset) = variables.get_offset(lhs) else {
                    return Err(CompileError::VariableNotDefine(lhs.clone()));
                };
                let Some(rhs_offset) = variables.get_offset(rhs) else {
                    return Err(CompileError::VariableNotDefine(rhs.clone()));
                };
                let lhs_ty = variables.get_type(lhs_offset).unwrap();
                let rhs_ty = variables.get_type(rhs_offset).unwrap();
                if lhs != rhs {
                    return Err(CompileError::TypeCheckError(rhs_ty.clone(), lhs_ty.clone()));
                }
                constraints.insert(CrossConstraint {
                    op: rule::Op::Equ,
                    lhs: VarId(lhs_offset),
                    rhs: VarId(rhs_offset),
                });
            }
            Head::LetEq(Pattern::Atom(a), Pattern::Variable(var))
            | Head::LetEq(Pattern::Variable(var), Pattern::Atom(a)) => {
                let Some(var) = variables.get_offset(var) else {
                    return Err(CompileError::VariableNotDefine(var.clone()));
                };
                let defined_type = variables.get_type(var).unwrap();
                let atom_ty = a.get_type();
                let Type::Base(def_type) = defined_type else {
                    return Err(CompileError::TypeCheckError(
                        Type::Base(atom_ty),
                        defined_type.clone(),
                    ));
                };
                if def_type != &atom_ty {
                    return Err(CompileError::TypeCheckError(
                        Type::Base(atom_ty),
                        defined_type.clone(),
                    ));
                }
                guard_cmps
                    .entry(VarId(var))
                    .or_default()
                    .insert(Constraint {
                        op: rule::Op::Equ,
                        value: a.clone().to_value(),
                    });
            }
            Head::LetEq(pattern, pattern1) => {
                // FIXME: defined_type
                let defined_type = Type::Base(crate::types::BaseType::Id);
                let Some(pattern) = check_and_compile_pattern(
                    pattern,
                    &defined_type,
                    table_env,
                    &mut variables,
                    &mut scans,
                    &mut guard_cmps,
                    &mut constraints,
                )?
                else {
                    continue;
                };
                let Some(pattern1) = check_and_compile_pattern(
                    pattern1,
                    &defined_type,
                    table_env,
                    &mut variables,
                    &mut scans,
                    &mut guard_cmps,
                    &mut constraints,
                )?
                else {
                    continue;
                };
                constraints.insert(CrossConstraint {
                    op: rule::Op::Equ,
                    lhs: pattern,
                    rhs: pattern1,
                });
            }
        }
    }

    let var_cols = scans
        .into_iter()
        .map(|(var, fused_scans)| {
            fused_scans
                .into_iter()
                .map(|fused_scan| FusedScan {
                    constraints: guard_cmps
                        .get(&var)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .collect(),
                    ..fused_scan
                })
                .collect()
        })
        .collect();

    Ok(Query {
        variables,
        var_cols,
        constraints: constraints.into_iter().collect(),
    })
}

fn check_and_compile_con_pattern(
    constructor_pattern: &ConstructorPattern,
    table_env: &TableEnv,
    get_result: bool,
    variables: &mut VariableRecord,
    scans: &mut Map<VarId, Vec<FusedScan>>,
    guard_cmps: &mut Map<VarId, Set<Constraint>>,
    constraints: &mut Set<CrossConstraint>,
) -> Result<Option<VarId>, CompileError> {
    let offset = table_env
        .get_offset(&constructor_pattern.0)
        .ok_or_else(|| CompileError::InvalidTableName(constructor_pattern.0.clone()))?;
    let table = &table_env.tables[offset];

    if table.1.len() != constructor_pattern.1.len() {
        return Err(CompileError::InvalidTableWidth(
            constructor_pattern.1.len(),
            table.1.len(),
        ));
    }

    for (column, pattern) in constructor_pattern.1.iter().enumerate() {
        let ty = &table.1[column];
        let Some(var) = check_and_compile_pattern(
            pattern,
            ty,
            table_env,
            variables,
            scans,
            guard_cmps,
            constraints,
        )?
        else {
            continue;
        };
        scans.entry(var).or_default().push(FusedScan {
            table: offset,
            column: ColumnIndex(column),
            column_type: ty.clone(),
            constraints: Box::new([]),
        });
    }

    if get_result {
        let column_type = table
            .2
            .as_ref()
            .cloned()
            .unwrap_or(Type::Base(crate::types::BaseType::Id));
        let res = variables.insert_var(None, column_type.clone());
        scans.entry(res).or_default().push(FusedScan {
            table: offset,
            column: ColumnIndex(table.1.len()),
            column_type,
            constraints: Box::new([]),
        });
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

fn check_and_compile_pattern(
    pattern: &Pattern,
    defined_type: &Type,
    table_env: &TableEnv,
    variables: &mut VariableRecord,
    scans: &mut Map<VarId, Vec<FusedScan>>,
    guard_cmps: &mut Map<VarId, Set<Constraint>>,
    constraints: &mut Set<CrossConstraint>,
) -> Result<Option<VarId>, CompileError> {
    match pattern {
        Pattern::Wildcard => Ok(None),
        Pattern::Atom(atom) => {
            let Type::Base(def_ty) = defined_type else {
                return Err(CompileError::InvalidAtomType(
                    atom.clone(),
                    defined_type.clone(),
                ));
            };
            if &atom.get_type() != def_ty {
                return Err(CompileError::InvalidAtomType(
                    atom.clone(),
                    defined_type.clone(),
                ));
            }

            Ok(Some(variables.insert_var(None, defined_type.clone())))
        }
        Pattern::Variable(name) => {
            if let Some(var) = variables.get_offset(name) {
                Ok(Some(VarId(var)))
            } else {
                Ok(Some(
                    variables.insert_var(Some(name.clone()), defined_type.clone()),
                ))
            }
        }
        Pattern::Constructor(constructor_pattern) => {
            let res = check_and_compile_con_pattern(
                constructor_pattern,
                table_env,
                true,
                variables,
                scans,
                guard_cmps,
                constraints,
            )?;
            Ok(Some(res.unwrap()))
        }
    }
}
