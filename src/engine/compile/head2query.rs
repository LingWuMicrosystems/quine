use crate::{
    engine::{
        compile::atom_to_value,
        env::{DataTypeEnv, TableEnv},
        error::CompileError,
        interner::Interner,
    },
    syntax::{AtomOrVariable, ConstructorPattern, Head, Pattern},
};

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
                    .insert((*op, atom_to_value(a, ctx.interner)));
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
                    .insert((Op::Equ, atom_to_value(a, ctx.interner)));
            }
            Head::LetEq(_, pattern, pattern1) => {
                let defined_type = Type::Base(BaseType::Id);
                let Some(lhs) = check_and_compile_pattern(&mut ctx, pattern, &defined_type, None)?
                else {
                    continue;
                };
                let Some(rhs) = check_and_compile_pattern(&mut ctx, pattern1, &defined_type, None)?
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

    let arity = table.1.len().saturating_sub(1);
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
        let result_ty = table.1.last().cloned().unwrap_or(Type::Base(BaseType::Id));
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
                        value: atom_to_value(atom, ctx.interner),
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
            let Type::Name(expected_name) = defined_type else {
                return Err(CompileError::TypeCheckError(
                    defined_type.clone(),
                    Type::Name(constructor_pattern.name.clone()),
                ));
            };
            let cons_name = format!("{}.{}", expected_name, constructor_pattern.name);
            let cons_type = ctx
                .data_types
                .get_constructor_type(&cons_name)
                .ok_or_else(|| CompileError::InvalidTableName(constructor_pattern.name.clone()))?;
            if cons_type != *expected_name {
                return Err(CompileError::TypeCheckError(
                    Type::Name(expected_name.clone()),
                    Type::Name(cons_type.clone()),
                ));
            }
            let res = check_and_compile_con_pattern(ctx, constructor_pattern, true)?;
            Ok(Some(res.unwrap()))
        }
    }
}
