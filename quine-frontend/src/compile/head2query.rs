use crate::{
    compile::atom_to_value,
    env::{DataTypeEnv, TableEnv},
    error::CompileError,
    interner::Interner,
    syntax::{AtomOrVariable, ConstructorPattern, Head, Pattern},
};

use alloc::vec::Vec;
use quine_core::{common::*, rule::*, types::*};

struct QueryCtx<'a> {
    table_env: &'a TableEnv,
    data_types: &'a DataTypeEnv,
    variables: &'a mut VariableRecord,
    scan_steps: &'a mut Vec<ScanStep>,
    var_to_steps: &'a mut Map<VarId, Vec<usize>>,
    guard_cmps: &'a mut Map<VarId, Set<(Op, Value)>>,
    constraints: &'a mut Set<CrossConstraint>,
    interner: &'a mut Interner,
    current_step: usize,
}

pub fn heads2query(
    heads: &[Head],
    table_env: &TableEnv,
    data_types: &DataTypeEnv,
    interner: &mut Interner,
) -> Result<Query, CompileError> {
    let mut variables = VariableRecord::default();
    let mut scan_steps: Vec<ScanStep> = Vec::new();
    let mut var_to_steps: Map<VarId, Vec<usize>> = Map::default();
    let mut guard_cmps: Map<VarId, Set<(Op, Value)>> = Map::default();
    let mut constraints: Set<CrossConstraint> = Set::default();

    let mut ctx = QueryCtx {
        table_env,
        data_types,
        variables: &mut variables,
        scan_steps: &mut scan_steps,
        var_to_steps: &mut var_to_steps,
        guard_cmps: &mut guard_cmps,
        constraints: &mut constraints,
        interner,
        current_step: usize::MAX,
    };

    for head in heads {
        match head {
            Head::Match(_, constructor_pattern) => {
                check_and_compile_con_pattern(&mut ctx, constructor_pattern, false)?;
            }
            Head::Guard(_, op, var, AtomOrVariable::Atom(a)) => {
                let Some(offset) = ctx.variables.get_offset(var) else {
                    return Err(CompileError::VariableNotDefine(var.clone()));
                };
                let defined_type = ctx.variables.get_type(offset).unwrap();
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
                ctx.guard_cmps
                    .entry(VarId(offset))
                    .or_default()
                    .insert((*op, atom_to_value(a.clone(), ctx.interner)));
            }
            Head::Guard(_, op, lhs, AtomOrVariable::Variable(rhs)) => {
                let Some(lhs_offset) = ctx.variables.get_offset(lhs) else {
                    return Err(CompileError::VariableNotDefine(lhs.clone()));
                };
                let Some(rhs_offset) = ctx.variables.get_offset(rhs) else {
                    return Err(CompileError::VariableNotDefine(rhs.clone()));
                };
                let lhs_ty = ctx.variables.get_type(lhs_offset).unwrap();
                let rhs_ty = ctx.variables.get_type(rhs_offset).unwrap();
                if lhs_ty != rhs_ty {
                    return Err(CompileError::TypeCheckError(rhs_ty.clone(), lhs_ty.clone()));
                }
                ctx.constraints.insert(CrossConstraint {
                    op: *op,
                    lhs: VarId(lhs_offset),
                    rhs: VarId(rhs_offset),
                });
            }
            Head::LetEq(_, Pattern::Variable(_, lhs), Pattern::Variable(_, rhs)) => {
                let Some(lhs_offset) = ctx.variables.get_offset(lhs) else {
                    return Err(CompileError::VariableNotDefine(lhs.clone()));
                };
                let Some(rhs_offset) = ctx.variables.get_offset(rhs) else {
                    return Err(CompileError::VariableNotDefine(rhs.clone()));
                };
                let lhs_ty = ctx.variables.get_type(lhs_offset).unwrap();
                let rhs_ty = ctx.variables.get_type(rhs_offset).unwrap();
                if lhs_ty != rhs_ty {
                    return Err(CompileError::TypeCheckError(rhs_ty.clone(), lhs_ty.clone()));
                }
                ctx.constraints.insert(CrossConstraint {
                    op: Op::Equ,
                    lhs: VarId(lhs_offset),
                    rhs: VarId(rhs_offset),
                });
            }
            Head::LetEq(_, Pattern::Atom(_, a), Pattern::Variable(_, var))
            | Head::LetEq(_, Pattern::Variable(_, var), Pattern::Atom(_, a)) => {
                let Some(offset) = ctx.variables.get_offset(var) else {
                    return Err(CompileError::VariableNotDefine(var.clone()));
                };
                let defined_type = ctx.variables.get_type(offset).unwrap();
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
                ctx.guard_cmps
                    .entry(VarId(offset))
                    .or_default()
                    .insert((Op::Equ, atom_to_value(a.clone(), ctx.interner)));
            }
            Head::LetEq(_, pattern, pattern1) => {
                let t1 = infer_data_type(pattern, &ctx);
                let t2 = infer_data_type(pattern1, &ctx);
                let defined_type = match (t1, t2) {
                    (Some(a), Some(b)) => {
                        crate::compile::unify(&a, &b)?;
                        a
                    }
                    (Some(t), None) | (None, Some(t)) => t,
                    (None, None) => Type::Base(BaseType::Id),
                };
                let Some(rhs) = check_and_compile_pattern(&mut ctx, pattern1, &defined_type, None)?
                else {
                    continue;
                };
                if let Pattern::Variable(_, name) = pattern {
                    if let Some(existing) = ctx.variables.get_offset(name) {
                        ctx.constraints.insert(CrossConstraint {
                            op: Op::Equ,
                            lhs: VarId(existing),
                            rhs,
                        });
                    } else {
                        ctx.variables.names_map.insert(name.clone(), rhs.0);
                    }
                } else {
                    let Some(lhs) =
                        check_and_compile_pattern(&mut ctx, pattern, &defined_type, None)?
                    else {
                        continue;
                    };
                    ctx.constraints.insert(CrossConstraint {
                        op: Op::Equ,
                        lhs,
                        rhs,
                    });
                }
            }
        }
    }

    // push guard_cmps down into scan step constraints
    let guard_cmps = core::mem::take(ctx.guard_cmps);
    for (var, cmps) in guard_cmps {
        if let Some(step_indices) = ctx.var_to_steps.get(&var) {
            for &step_idx in step_indices {
                let step = &mut ctx.scan_steps[step_idx];
                if let Some((col, _)) = step.columns.iter().find(|(_, v)| *v == var) {
                    for &(op, value) in &cmps {
                        step.constraints.push(Constraint {
                            op,
                            column: *col,
                            value,
                        });
                    }
                }
            }
        }
    }

    Ok(Query {
        variables,
        scan_steps: scan_steps.into(),
        constraints: constraints.into_iter().collect(),
    })
}

fn check_and_compile_con_pattern(
    ctx: &mut QueryCtx,
    constructor_pattern: &ConstructorPattern,
    get_result: bool,
) -> Result<Option<VarId>, CompileError> {
    let table_id = ctx
        .table_env
        .get_offset(&constructor_pattern.name)
        .ok_or_else(|| CompileError::InvalidTableName(constructor_pattern.name.clone()))?;
    let table = &ctx.table_env.tables[table_id];

    let arity = table.1.len() - 1;
    if arity != constructor_pattern.args.len() {
        return Err(CompileError::InvalidTableWidth(
            constructor_pattern.args.len(),
            arity,
        ));
    }

    let step_idx = ctx.scan_steps.len();
    let prev_step = ctx.current_step;
    ctx.current_step = step_idx;
    ctx.scan_steps.push(ScanStep {
        table: table_id,
        columns: Vec::new(),
        constraints: Vec::new(),
    });

    for (col, pattern) in constructor_pattern.args.iter().enumerate() {
        let ty = &table.1[col];
        if let Some(var) = check_and_compile_pattern(ctx, pattern, ty, Some(ColumnIndex(col)))? {
            ctx.scan_steps[step_idx]
                .columns
                .push((ColumnIndex(col), var));
            ctx.var_to_steps.entry(var).or_default().push(step_idx);
        }
    }

    if get_result {
        let result_col = ColumnIndex(arity);
        let result_ty = ctx
            .data_types
            .get_constructor_type(&constructor_pattern.name)
            .map(Type::Name)
            .unwrap_or_else(|| table.1[arity].clone());
        let res = ctx.variables.insert_var(None, result_ty);
        ctx.scan_steps[step_idx].columns.push((result_col, res));
        ctx.var_to_steps.entry(res).or_default().push(step_idx);
        ctx.current_step = prev_step;
        Ok(Some(res))
    } else {
        ctx.current_step = prev_step;
        Ok(None)
    }
}

fn infer_data_type(pattern: &Pattern, ctx: &QueryCtx) -> Option<Type> {
    match pattern {
        Pattern::Constructor(_, cp) => {
            if let Some(type_name) = ctx.data_types.get_constructor_type(&cp.name) {
                Some(Type::Name(type_name))
            } else if let Some(table_def) = ctx.table_env.get_from_name(&cp.name) {
                let arity = table_def.1.len() - 1;
                Some(table_def.1[arity].clone())
            } else {
                None
            }
        }
        Pattern::Variable(_, name) => ctx
            .variables
            .get_offset(name)
            .and_then(|o| ctx.variables.get_type(o))
            .cloned(),
        Pattern::Atom(_, a) => Some(Type::Base(a.get_type())),
        _ => None,
    }
}

fn check_and_compile_pattern(
    ctx: &mut QueryCtx,
    pattern: &Pattern,
    defined_type: &Type,
    column: Option<ColumnIndex>,
) -> Result<Option<VarId>, CompileError> {
    match pattern {
        Pattern::Wildcard(_) => Ok(None),
        Pattern::Atom(_, atom) => {
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
            let var = ctx.variables.insert_var(None, defined_type.clone());
            if let Some(col) = column {
                ctx.scan_steps[ctx.current_step]
                    .constraints
                    .push(Constraint {
                        op: Op::Equ,
                        column: col,
                        value: atom_to_value(atom.clone(), ctx.interner),
                    });
            }
            Ok(Some(var))
        }
        Pattern::Variable(_, name) => {
            if let Some(offset) = ctx.variables.get_offset(name) {
                Ok(Some(VarId(offset)))
            } else {
                Ok(Some(
                    ctx.variables
                        .insert_var(Some(name.clone()), defined_type.clone()),
                ))
            }
        }
        Pattern::Constructor(_, constructor_pattern) => {
            if let Some(actual_type) =
                ctx.data_types.get_constructor_type(&constructor_pattern.name)
            {
                let expected = Type::Name(actual_type);
                if &expected != defined_type {
                    return Err(CompileError::TypeCheckError(expected, defined_type.clone()));
                }
                let res = check_and_compile_con_pattern(ctx, constructor_pattern, true)?;
                Ok(Some(res.unwrap()))
            } else if let Some(table_def) =
                ctx.table_env.get_from_name(&constructor_pattern.name)
            {
                let arity = table_def.1.len() - 1;
                let actual_type = table_def.1[arity].clone();
                if &actual_type != defined_type {
                    return Err(CompileError::TypeCheckError(
                        actual_type,
                        defined_type.clone(),
                    ));
                }
                let res = check_and_compile_con_pattern(ctx, constructor_pattern, true)?;
                Ok(Some(res.unwrap()))
            } else {
                Err(CompileError::InvalidTableName(
                    constructor_pattern.name.clone(),
                ))
            }
        }
    }
}
