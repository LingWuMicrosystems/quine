use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{
    engine::{
        env::TableEnv,
        error::CompileError,
        frontend::{
            syntax::{AtomOrVariable, ConstructorPattern, Head, Pattern},
            utils::atom_to_value,
        },
        interner::Interner,
    },
    regraph::{
        common::{ColumnIndex, Map, Set, VarId},
        rule::{self, Constraint, CrossConstraint, FusedScan, Op, Query, VariableRecord},
        types::{BaseType, Type},
    },
};

struct QueryCtx<'a> {
    table_env: &'a TableEnv,
    variables: &'a mut VariableRecord,
    scans: &'a mut Map<VarId, Vec<FusedScan>>,
    guard_cmps: &'a mut Map<VarId, Set<Constraint>>,
    constraints: &'a mut Set<CrossConstraint>,
    interner: &'a mut Interner,
}

pub fn heads2query(
    heads: &[Head],
    table_env: &TableEnv,
    interner: &mut Interner,
) -> Result<Query, CompileError> {
    let mut variables = VariableRecord::default();
    let mut scans: Map<VarId, Vec<FusedScan>> = Map::default();
    let mut guard_cmps: Map<VarId, Set<Constraint>> = Map::default();
    let mut constraints: Set<CrossConstraint> = Set::default();

    let mut ctx = QueryCtx {
        table_env,
        variables: &mut variables,
        scans: &mut scans,
        guard_cmps: &mut guard_cmps,
        constraints: &mut constraints,
        interner,
    };

    for head in heads {
        match head {
            Head::Match(constructor_pattern) => {
                check_and_compile_con_pattern(&mut ctx, constructor_pattern, false)?;
            }
            Head::Guard(op, var, AtomOrVariable::Atom(a)) => {
                let Some(var) = ctx.variables.get_offset(var) else {
                    return Err(CompileError::VariableNotDefine(var.clone()));
                };
                let defined_type = ctx.variables.get_type(var).unwrap();
                let atom_ty = a.get_type();
                let Type::Base(def_type) = defined_type else {
                    return Err(CompileError::TypeCheckError(Type::Base(atom_ty), defined_type.clone()));
                };
                if def_type != &atom_ty {
                    return Err(CompileError::TypeCheckError(Type::Base(atom_ty), defined_type.clone()));
                }
                ctx.guard_cmps.entry(VarId(var)).or_default().insert(Constraint {
                    op: *op, value: atom_to_value(a, ctx.interner),
                });
            }
            Head::Guard(op, lhs, AtomOrVariable::Variable(rhs)) => {
                let Some(lhs_offset) = ctx.variables.get_offset(lhs) else {
                    return Err(CompileError::VariableNotDefine(lhs.clone()));
                };
                let Some(rhs_offset) = ctx.variables.get_offset(rhs) else {
                    return Err(CompileError::VariableNotDefine(rhs.clone()));
                };
                let lhs_ty = ctx.variables.get_type(lhs_offset).unwrap();
                let rhs_ty = ctx.variables.get_type(rhs_offset).unwrap();
                if lhs != rhs {
                    return Err(CompileError::TypeCheckError(rhs_ty.clone(), lhs_ty.clone()));
                }
                ctx.constraints.insert(CrossConstraint {
                    op: *op, lhs: VarId(lhs_offset), rhs: VarId(rhs_offset),
                });
            }
            Head::LetEq(Pattern::Variable(lhs), Pattern::Variable(rhs)) => {
                let Some(lhs_offset) = ctx.variables.get_offset(lhs) else {
                    return Err(CompileError::VariableNotDefine(lhs.clone()));
                };
                let Some(rhs_offset) = ctx.variables.get_offset(rhs) else {
                    return Err(CompileError::VariableNotDefine(rhs.clone()));
                };
                let lhs_ty = ctx.variables.get_type(lhs_offset).unwrap();
                let rhs_ty = ctx.variables.get_type(rhs_offset).unwrap();
                if lhs != rhs {
                    return Err(CompileError::TypeCheckError(rhs_ty.clone(), lhs_ty.clone()));
                }
                ctx.constraints.insert(CrossConstraint {
                    op: rule::Op::Equ, lhs: VarId(lhs_offset), rhs: VarId(rhs_offset),
                });
            }
            Head::LetEq(Pattern::Atom(a), Pattern::Variable(var))
            | Head::LetEq(Pattern::Variable(var), Pattern::Atom(a)) => {
                let Some(var) = ctx.variables.get_offset(var) else {
                    return Err(CompileError::VariableNotDefine(var.clone()));
                };
                let defined_type = ctx.variables.get_type(var).unwrap();
                let atom_ty = a.get_type();
                let Type::Base(def_type) = defined_type else {
                    return Err(CompileError::TypeCheckError(Type::Base(atom_ty), defined_type.clone()));
                };
                if def_type != &atom_ty {
                    return Err(CompileError::TypeCheckError(Type::Base(atom_ty), defined_type.clone()));
                }
                ctx.guard_cmps.entry(VarId(var)).or_default().insert(Constraint {
                    op: rule::Op::Equ, value: atom_to_value(a, ctx.interner),
                });
            }
            Head::LetEq(pattern, pattern1) => {
                let defined_type = Type::Base(BaseType::Id);
                let Some(pattern) = check_and_compile_pattern(&mut ctx, pattern, &defined_type)?
                else { continue };
                let Some(pattern1) = check_and_compile_pattern(&mut ctx, pattern1, &defined_type)?
                else { continue };
                ctx.constraints.insert(CrossConstraint {
                    op: rule::Op::Equ, lhs: pattern, rhs: pattern1,
                });
            }
        }
    }

    let var_cols = scans.into_iter()
        .map(|(var, fused_scans)| fused_scans.into_iter()
            .map(|fused_scan| FusedScan {
                constraints: guard_cmps.get(&var).cloned().unwrap_or_default().into_iter().collect(),
                ..fused_scan
            })
            .collect())
        .collect();

    Ok(Query { variables, var_cols, constraints: constraints.into_iter().collect() })
}

fn check_and_compile_con_pattern(
    ctx: &mut QueryCtx,
    constructor_pattern: &ConstructorPattern,
    get_result: bool,
) -> Result<Option<VarId>, CompileError> {
    let offset = ctx.table_env
        .get_offset(&constructor_pattern.0)
        .ok_or_else(|| CompileError::InvalidTableName(constructor_pattern.0.clone()))?;
    let table = &ctx.table_env.tables[offset];

    if table.1.len() != constructor_pattern.1.len() {
        return Err(CompileError::InvalidTableWidth(constructor_pattern.1.len(), table.1.len()));
    }

    for (column, pattern) in constructor_pattern.1.iter().enumerate() {
        let ty = &table.1[column];
        let Some(var) = check_and_compile_pattern(ctx, pattern, ty)? else { continue };
        ctx.scans.entry(var).or_default().push(FusedScan {
            table: offset, column: ColumnIndex(column),
            column_type: ty.clone(), constraints: Box::new([]),
        });
    }

    if get_result {
        let column_type = table.2.as_ref().cloned().unwrap_or(Type::Base(BaseType::Id));
        let res = ctx.variables.insert_var(None, column_type.clone());
        ctx.scans.entry(res).or_default().push(FusedScan {
            table: offset, column: ColumnIndex(table.1.len()),
            column_type, constraints: Box::new([]),
        });
        Ok(Some(res))
    } else {
        Ok(None)
    }
}

fn check_and_compile_pattern(
    ctx: &mut QueryCtx,
    pattern: &Pattern,
    defined_type: &Type,
) -> Result<Option<VarId>, CompileError> {
    match pattern {
        Pattern::Wildcard => Ok(None),
        Pattern::Atom(atom) => {
            let Type::Base(def_ty) = defined_type else {
                return Err(CompileError::InvalidAtomType(atom.clone(), defined_type.clone()));
            };
            if &atom.get_type() != def_ty {
                return Err(CompileError::InvalidAtomType(atom.clone(), defined_type.clone()));
            }
            let varid = ctx.variables.insert_var(None, defined_type.clone());
            ctx.guard_cmps.entry(varid).or_default().insert(Constraint {
                op: Op::Equ, value: atom_to_value(atom, ctx.interner),
            });
            Ok(Some(varid))
        }
        Pattern::Variable(name) => {
            if let Some(var) = ctx.variables.get_offset(name) {
                Ok(Some(VarId(var)))
            } else {
                Ok(Some(ctx.variables.insert_var(Some(name.clone()), defined_type.clone())))
            }
        }
        Pattern::Constructor(constructor_pattern) => {
            let res = check_and_compile_con_pattern(ctx, constructor_pattern, true)?;
            Ok(Some(res.unwrap()))
        }
    }
}
