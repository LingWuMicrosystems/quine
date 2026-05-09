use alloc::vec::Vec;
use alloc::{boxed::Box, vec};

use crate::common::Set;
use crate::core::rule::{Constraint, VarColsScanRule};
use crate::{
    common::{Map, Variable},
    core::{
        command::BackendCommand,
        rule::{self, FusedScan, Query, VariableRecord},
    },
    frontend::{
        body2action::{body2action, function_call2action},
        env::{DataTypeEnv, TableEnv},
        error::CompileError,
        head2flat_clause::{FlatClause, heads2flat_clause},
        syntax::{self, Command, Head, VarName},
    },
    types::{TableDef, Type},
};

pub type ResolvedClause = FlatClause<(Variable, Type)>;

#[derive(Debug, Default, Clone)]
pub struct CompileEnv {
    pub data_types: DataTypeEnv,
    pub table_types: TableEnv,
}

impl CompileEnv {
    pub fn check_and_compile_command(
        &mut self,
        command: Command,
    ) -> Result<BackendCommand, CompileError> {
        match command {
            Command::TypeDef(name, type_def) => {
                self.data_types.insert(name, type_def)?;
                // todo
                Ok(BackendCommand::AddTables(vec![]))
            }
            Command::TableDef(name, table_def) => {
                let table_def = TableDef(
                    table_def.0,
                    table_def.1.iter().map(Type::to_base_type).collect(),
                    table_def.2.as_ref().map(Type::to_base_type),
                );
                self.table_types.insert(name, table_def.clone())?;
                Ok(BackendCommand::AddTables(vec![table_def]))
            }
            Command::Rule(rule) => Ok(BackendCommand::AddRule(self.check_and_compile_rule(&rule)?)),
            Command::Fact(fact) => Ok(BackendCommand::Actions(function_call2action(
                &fact,
                &Map::default(),
            ))),
            Command::Query(head) => Ok(BackendCommand::Query(self.check_and_compile_query(&head)?)),
        }
    }

    fn check_and_compile_rule(&self, rule: &syntax::Rule) -> Result<rule::Rule, CompileError> {
        let query = self.check_and_compile_query(&rule.heads)?;
        let action = body2action(&rule.bodys, &query.variables);
        Ok(rule::Rule { query, action })
    }

    fn check_and_compile_query(&self, head: &[Head]) -> Result<Query, CompileError> {
        let (clauses, variables) = self.check_and_compile_heads(&head)?;

        let var_cols: Box<[VarColsScanRule]> = (0..variables.len())
            .into_iter()
            .map(|offset| self.collect_scans(offset, &clauses))
            .collect::<Result<Box<[_]>, _>>()?;

        let constraints: Set<_> = clauses
            .into_iter()
            .filter_map(|clause| {
                if let FlatClause::Guard(op, (lhs, ty0), (rhs, ty1)) = clause {
                    debug_assert_eq!(ty0, ty1);
                    Some(rule::CrossConstraint {
                        op: op.to_constraint_op(ty0.is_sign()),
                        lhs,
                        rhs,
                    })
                } else {
                    None
                }
            })
            .collect();
        let constraints = constraints.into_iter().collect();

        Ok(Query {
            variables,
            var_cols,
            constraints,
        })
    }

    fn collect_scans(
        &self,
        offset: usize,
        clauses: &[ResolvedClause],
    ) -> Result<VarColsScanRule, CompileError> {
        let mut scans = Set::default();
        for clause in clauses {
            if let FlatClause::Lookup((var, ty), table, column) = clause
                && offset == var.0
            {
                let table_offset = self
                    .table_types
                    .get_offset(&table)
                    .ok_or_else(|| CompileError::InvalidTableName(table.clone()))?;

                let cs = clauses
                    .iter()
                    .filter_map(|clause| {
                        if let FlatClause::ConstCompare(op, (var1, _), a) = clause
                            && var1.0 == var.0
                        {
                            Some(Constraint {
                                op: op.to_constraint_op(ty.is_sign()),
                                value: a.clone().to_value(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Set<_>>();

                scans.insert(FusedScan {
                    table: table_offset,
                    column: *column,
                    column_type: ty.clone(),
                    constraints: cs.into_iter().collect(),
                });
            }
        }
        Ok(scans.into_iter().collect())
    }

    fn check_and_compile_heads(
        &self,
        heads: &[Head],
    ) -> Result<(Vec<ResolvedClause>, VariableRecord), CompileError> {
        let clauses = heads2flat_clause(&heads)?;
        let mut var_record = Vec::default();
        let mut resolved_clauses = Vec::default();
        for c in clauses {
            resolved_clauses.push(self.clause_check(&c, &mut var_record)?);
        }
        Ok((resolved_clauses, var_record))
    }

    fn clause_check(
        &self,
        clause: &FlatClause<VarName>,
        var_record: &mut VariableRecord,
    ) -> Result<ResolvedClause, CompileError> {
        match clause {
            FlatClause::Lookup(var, table_name, column_index) => {
                let table = self
                    .table_types
                    .get_from_name(table_name)
                    .ok_or_else(|| CompileError::InvalidTableName(table_name.clone()))?;
                if column_index.0 > table.1.len() {
                    return Err(CompileError::InvalidTableName(table_name.clone()));
                }
                let ty = if column_index.0 == table.1.len() {
                    table.2.clone()
                } else {
                    table.1.get(column_index.0).cloned()
                }
                .ok_or_else(|| {
                    CompileError::InvalidTableColumn(table_name.clone(), *column_index)
                })?;
            }
            FlatClause::ConstCompare(op, _, atom) => todo!(),
            FlatClause::Guard(op, _, _) => todo!(),
        }
        todo!()
    }
}
