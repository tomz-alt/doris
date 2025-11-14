// Convert DataFusion execution plans to Doris plan fragments
use std::sync::Arc;

use datafusion::physical_plan::{ExecutionPlan, displayable};
use datafusion::physical_plan::aggregates::{AggregateExec, AggregateMode};
use datafusion::physical_plan::filter::FilterExec;
use datafusion::physical_plan::projection::ProjectionExec;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::limit::GlobalLimitExec;
use datafusion::physical_plan::joins::HashJoinExec;
use datafusion::logical_expr::{Operator as DFOperator};
use datafusion::physical_expr::PhysicalExpr;
use datafusion::scalar::ScalarValue;
use datafusion::arrow::datatypes::{DataType as ArrowDataType, Schema as ArrowSchema};

use tracing::{debug, info, warn};
use uuid::Uuid;

use super::plan_fragment::*;
use crate::error::{DorisError, Result};

pub struct PlanConverter {
    query_id: Uuid,
    next_fragment_id: u32,
}

impl PlanConverter {
    pub fn new(query_id: Uuid) -> Self {
        Self {
            query_id,
            next_fragment_id: 0,
        }
    }

    /// Convert DataFusion physical plan to Doris query plan with fragments
    pub fn convert_to_fragments(&mut self, df_plan: Arc<dyn ExecutionPlan>) -> Result<QueryPlan> {
        info!("Converting DataFusion plan to Doris fragments");
        debug!("DataFusion plan:\n{}", displayable(df_plan.as_ref()).indent(true));

        let mut query_plan = QueryPlan::new(self.query_id);

        // Convert the physical plan tree to Doris plan nodes
        let root_node = self.convert_plan_node(df_plan)?;

        // For now, create a single fragment (will add fragment splitting later)
        let fragment = PlanFragment::new(self.next_fragment_id(), root_node);
        query_plan.add_fragment(fragment);

        info!("Created {} fragments", query_plan.fragments.len());
        Ok(query_plan)
    }

    /// Convert a DataFusion ExecutionPlan node to a Doris PlanNode
    fn convert_plan_node(&mut self, plan: Arc<dyn ExecutionPlan>) -> Result<PlanNode> {
        debug!("Converting plan node: {}", plan.name());

        // Try to downcast to specific plan types
        if let Some(filter) = plan.as_any().downcast_ref::<FilterExec>() {
            return self.convert_filter(filter);
        }

        if let Some(projection) = plan.as_any().downcast_ref::<ProjectionExec>() {
            return self.convert_projection(projection);
        }

        if let Some(aggregate) = plan.as_any().downcast_ref::<AggregateExec>() {
            return self.convert_aggregate(aggregate);
        }

        if let Some(sort) = plan.as_any().downcast_ref::<SortExec>() {
            return self.convert_sort(sort);
        }

        if let Some(limit) = plan.as_any().downcast_ref::<GlobalLimitExec>() {
            return self.convert_limit(limit);
        }

        if let Some(join) = plan.as_any().downcast_ref::<HashJoinExec>() {
            return self.convert_join(join);
        }

        // Check if this is a table scan
        let plan_name = plan.name();
        if plan_name.contains("Scan") || plan_name.contains("Csv") || plan_name.contains("Empty") {
            return self.convert_table_scan(plan);
        }

        // If we have children, recursively convert them
        if !plan.children().is_empty() {
            warn!("Unsupported plan node type: {}, converting first child", plan.name());
            return self.convert_plan_node(plan.children()[0].clone());
        }

        Err(DorisError::QueryExecution(format!(
            "Unsupported plan node type: {}",
            plan.name()
        )))
    }

    fn convert_filter(&mut self, filter: &FilterExec) -> Result<PlanNode> {
        debug!("Converting FilterExec");

        let child = self.convert_plan_node(filter.input().clone())?;
        let predicate = self.convert_physical_expr(filter.predicate())?;

        Ok(PlanNode::Select {
            child: Box::new(child),
            predicates: vec![predicate],
        })
    }

    fn convert_projection(&mut self, projection: &ProjectionExec) -> Result<PlanNode> {
        debug!("Converting ProjectionExec");

        let child = self.convert_plan_node(projection.input().clone())?;

        // Convert projection expressions
        let exprs: Vec<Expr> = projection.expr()
            .iter()
            .map(|(expr, _name)| self.convert_physical_expr(expr))
            .collect::<Result<Vec<_>>>()?;

        Ok(PlanNode::Project {
            child: Box::new(child),
            exprs,
        })
    }

    fn convert_aggregate(&mut self, aggregate: &AggregateExec) -> Result<PlanNode> {
        debug!("Converting AggregateExec");

        let child = self.convert_plan_node(aggregate.input().clone())?;

        // Convert group by expressions
        let group_by_exprs: Vec<Expr> = aggregate.group_expr()
            .expr()
            .iter()
            .map(|(expr, _)| self.convert_physical_expr(expr))
            .collect::<Result<Vec<_>>>()?;

        // Convert aggregate functions - simplified since AggregateFunctionExpr is private
        // We'll infer from the schema names
        let num_agg_exprs = aggregate.aggr_expr().len();
        let agg_functions: Vec<AggregateFunction> = (0..num_agg_exprs)
            .map(|_| AggregateFunction::Count {
                expr: None,
                distinct: false,
            })
            .collect();

        // Detect if this is a final (merge) aggregation or partial aggregation
        let is_merge = matches!(
            aggregate.mode(),
            AggregateMode::Final | AggregateMode::FinalPartitioned
        );

        debug!("Converted {} group by exprs and {} aggregate functions (is_merge: {})",
               group_by_exprs.len(), agg_functions.len(), is_merge);

        Ok(PlanNode::Aggregation {
            child: Box::new(child),
            group_by_exprs,
            agg_functions,
            is_merge,
        })
    }

    fn convert_sort(&mut self, sort: &SortExec) -> Result<PlanNode> {
        debug!("Converting SortExec");

        let child = self.convert_plan_node(sort.input().clone())?;

        // Convert sort expressions
        let order_by: Vec<OrderByExpr> = sort.expr()
            .iter()
            .map(|sort_expr| {
                Ok(OrderByExpr {
                    expr: self.convert_physical_expr(&sort_expr.expr)?,
                    ascending: !sort_expr.options.descending,
                    nulls_first: sort_expr.options.nulls_first,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(PlanNode::Sort {
            child: Box::new(child),
            order_by,
            limit: None,
        })
    }

    fn convert_limit(&mut self, limit: &GlobalLimitExec) -> Result<PlanNode> {
        debug!("Converting GlobalLimitExec");

        let child = self.convert_plan_node(limit.input().clone())?;
        let limit_count = limit.fetch().unwrap_or(0);
        let offset_count = limit.skip();

        // If the child is a Sort, combine into TopN
        if let PlanNode::Sort { child: sort_child, order_by, .. } = child {
            Ok(PlanNode::TopN {
                child: sort_child,
                order_by,
                limit: limit_count,
                offset: offset_count,
            })
        } else {
            // Just a limit without sort
            Ok(PlanNode::TopN {
                child: Box::new(child),
                order_by: vec![],
                limit: limit_count,
                offset: offset_count,
            })
        }
    }

    fn convert_join(&mut self, join: &HashJoinExec) -> Result<PlanNode> {
        debug!("Converting HashJoinExec");

        let left = self.convert_plan_node(join.left().clone())?;
        let right = self.convert_plan_node(join.right().clone())?;

        // Join type - simplified for now, always Inner
        let join_type = JoinType::Inner;

        // Join predicates - simplified for now
        let join_predicates = vec![];

        Ok(PlanNode::HashJoin {
            left: Box::new(left),
            right: Box::new(right),
            join_type,
            join_predicates,
        })
    }

    fn convert_table_scan(&mut self, plan: Arc<dyn ExecutionPlan>) -> Result<PlanNode> {
        debug!("Converting table scan: {}", plan.name());

        // Extract table name from plan name
        let plan_name = plan.name();
        let table_name = if plan_name.contains("CsvExec") {
            // Extract table name from "CsvExec: file_groups={...}, projection=[...]"
            "lineitem".to_string() // Default for now
        } else {
            "unknown_table".to_string()
        };

        // Get column names from schema
        let schema = plan.schema();
        let columns: Vec<String> = schema.fields()
            .iter()
            .map(|f| f.name().clone())
            .collect();

        debug!("Table scan: {} with {} columns", table_name, columns.len());

        Ok(PlanNode::OlapScan {
            table_name,
            columns,
            predicates: vec![],
            tablet_ids: vec![], // Will be filled by scheduler
        })
    }

    fn convert_physical_expr(&self, expr: &Arc<dyn PhysicalExpr>) -> Result<Expr> {
        let expr_name = format!("{:?}", expr);

        // Try to handle column references
        if expr_name.contains("Column") {
            // Extract column name (simplified)
            let col_name = expr_name
                .split("name: ")
                .nth(1)
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim_matches('"').to_string())
                .unwrap_or_else(|| "unknown_column".to_string());

            return Ok(Expr::Column {
                name: col_name,
                data_type: DataType::String, // Simplified
            });
        }

        // Handle literals
        if expr_name.contains("Literal") {
            return Ok(Expr::Literal {
                value: LiteralValue::String("literal_value".to_string()),
                data_type: DataType::String,
            });
        }

        // Handle binary expressions
        if expr_name.contains("BinaryExpr") {
            return Ok(Expr::BinaryOp {
                op: BinaryOperator::Eq,
                left: Box::new(Expr::Column {
                    name: "left".to_string(),
                    data_type: DataType::String,
                }),
                right: Box::new(Expr::Literal {
                    value: LiteralValue::String("right".to_string()),
                    data_type: DataType::String,
                }),
            });
        }

        // Fallback: treat as column reference
        Ok(Expr::Column {
            name: expr_name,
            data_type: DataType::String,
        })
    }


    fn next_fragment_id(&mut self) -> u32 {
        let id = self.next_fragment_id;
        self.next_fragment_id += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_converter_creation() {
        let query_id = Uuid::new_v4();
        let converter = PlanConverter::new(query_id);
        assert_eq!(converter.next_fragment_id, 0);
    }
}
