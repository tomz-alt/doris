// Fragment Executor - Coordinates distributed query execution across multiple BEs
// This is the final piece of Option B: sending fragments to BEs and merging results

use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::plan_fragment::*;
use super::feature_flags::QueryFeatureFlags;
use super::arrow_parser::ArrowResultParser;
use crate::error::{DorisError, Result};
use crate::be::BackendClientPool;
use crate::query::QueryResult;

/// Coordinates execution of multi-fragment distributed queries
pub struct FragmentExecutor {
    query_id: Uuid,
    feature_flags: Arc<QueryFeatureFlags>,
}

impl FragmentExecutor {
    pub fn new(query_id: Uuid) -> Self {
        Self::with_feature_flags(query_id, Arc::new(QueryFeatureFlags::default()))
    }

    pub fn with_feature_flags(query_id: Uuid, feature_flags: Arc<QueryFeatureFlags>) -> Self {
        Self {
            query_id,
            feature_flags,
        }
    }

    /// Execute a multi-fragment query plan on the BE cluster
    pub async fn execute_fragments(
        &self,
        query_plan: QueryPlan,
        be_pool: &BackendClientPool,
    ) -> Result<QueryResult> {
        info!(
            "Executing query {} with {} fragments",
            self.query_id,
            query_plan.fragments.len()
        );

        if query_plan.fragments.is_empty() {
            return Err(DorisError::QueryExecution(
                "Query plan has no fragments".to_string(),
            ));
        }

        // For now, execute a simplified version
        // In a full implementation, we would:
        // 1. Topologically sort fragments by dependencies
        // 2. Execute leaf fragments first (data fragments with scans)
        // 3. Execute intermediate fragments as data becomes available
        // 4. Execute coordinator fragment last
        // 5. Merge results according to exchange types

        // Simplified: Execute fragments in order and return last result
        let mut final_result = QueryResult::empty();

        for (idx, fragment) in query_plan.fragments.iter().enumerate() {
            debug!(
                "Executing fragment {} (ID: {})",
                idx, fragment.fragment_id
            );

            // Convert fragment to SQL (simplified - in production, send entire plan)
            let sql = self.fragment_to_sql(fragment);

            // Execute on BE
            match be_pool
                .execute_query(self.query_id, &sql)
                .await
            {
                Ok(result) => {
                    debug!(
                        "Fragment {} executed successfully: {} rows",
                        idx,
                        result.rows.len()
                    );
                    final_result = result;
                }
                Err(e) => {
                    warn!("Fragment {} execution failed: {}", idx, e);
                    // In a full implementation, handle partial failures
                    return Err(e);
                }
            }
        }

        info!("Query {} completed successfully", self.query_id);
        Ok(final_result)
    }

    /// Convert a fragment to SQL for execution (simplified)
    /// In production, we would serialize the entire PlanNode tree
    fn fragment_to_sql(&self, fragment: &PlanFragment) -> String {
        // Simplified: Extract basic SQL from fragment
        // In production, this would serialize the plan tree to protobuf

        match &fragment.root_node {
            PlanNode::OlapScan { table_name, .. } => {
                format!("SELECT * FROM {}", table_name)
            }
            PlanNode::Aggregation {
                child,
                group_by_exprs,
                agg_functions,
                is_merge: _,
            } => {
                // Generate aggregation SQL
                let agg_str = agg_functions
                    .iter()
                    .map(|f| match f {
                        AggregateFunction::Count { .. } => "COUNT(*)".to_string(),
                        AggregateFunction::Sum { expr, .. } => {
                            format!("SUM({})", expr_to_sql(expr))
                        }
                        AggregateFunction::Avg { expr, .. } => {
                            format!("AVG({})", expr_to_sql(expr))
                        }
                        AggregateFunction::Min { expr, .. } => {
                            format!("MIN({})", expr_to_sql(expr))
                        }
                        AggregateFunction::Max { expr, .. } => {
                            format!("MAX({})", expr_to_sql(expr))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let child_sql = self.fragment_to_sql_node(child);

                if group_by_exprs.is_empty() {
                    format!("SELECT {} FROM ({})", agg_str, child_sql)
                } else {
                    let group_by_str = group_by_exprs
                        .iter()
                        .map(expr_to_sql)
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "SELECT {}, {} FROM ({}) GROUP BY {}",
                        group_by_str, agg_str, child_sql, group_by_str
                    )
                }
            }
            PlanNode::TopN {
                child,
                order_by,
                limit,
                offset,
            } => {
                let child_sql = self.fragment_to_sql_node(child);
                let order_str = order_by
                    .iter()
                    .map(|o| {
                        format!(
                            "{} {}",
                            expr_to_sql(&o.expr),
                            if o.ascending { "ASC" } else { "DESC" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                if order_str.is_empty() {
                    format!(
                        "SELECT * FROM ({}) LIMIT {} OFFSET {}",
                        child_sql, limit, offset
                    )
                } else {
                    format!(
                        "SELECT * FROM ({}) ORDER BY {} LIMIT {} OFFSET {}",
                        child_sql, order_str, limit, offset
                    )
                }
            }
            _ => {
                // Fallback for other node types
                "SELECT 1".to_string()
            }
        }
    }

    fn fragment_to_sql_node(&self, node: &PlanNode) -> String {
        match node {
            PlanNode::OlapScan { table_name, .. } => table_name.clone(),
            PlanNode::Exchange { child, .. } => {
                // Exchange boundary - reference child fragment
                self.fragment_to_sql_node(child)
            }
            _ => "subquery".to_string(),
        }
    }
}

/// Convert expression to SQL string (simplified)
fn expr_to_sql(expr: &Expr) -> String {
    match expr {
        Expr::Column { name, .. } => name.clone(),
        Expr::Literal { value, .. } => literal_value_to_sql(value),
        Expr::BinaryOp { op, left, right } => {
            format!(
                "{} {} {}",
                expr_to_sql(left),
                binary_op_to_sql(op),
                expr_to_sql(right)
            )
        }
        Expr::UnaryOp { op, expr } => {
            format!("{} {}", unary_op_to_sql(op), expr_to_sql(expr))
        }
        Expr::Cast { expr, target_type } => {
            format!("CAST({} AS {:?})", expr_to_sql(expr), target_type)
        }
        Expr::Function { name, args } => {
            let args_str = args.iter().map(expr_to_sql).collect::<Vec<_>>().join(", ");
            format!("{}({})", name, args_str)
        }
    }
}

fn literal_value_to_sql(value: &LiteralValue) -> String {
    match value {
        LiteralValue::Null => "NULL".to_string(),
        LiteralValue::Boolean(b) => b.to_string(),
        LiteralValue::Int8(i) => i.to_string(),
        LiteralValue::Int16(i) => i.to_string(),
        LiteralValue::Int32(i) => i.to_string(),
        LiteralValue::Int64(i) => i.to_string(),
        LiteralValue::Float32(f) => f.to_string(),
        LiteralValue::Float64(f) => f.to_string(),
        LiteralValue::String(s) => format!("'{}'", s),
        LiteralValue::Date(d) => format!("'{}'", d),
        LiteralValue::DateTime(dt) => format!("'{}'", dt),
    }
}

fn binary_op_to_sql(op: &BinaryOperator) -> &str {
    match op {
        BinaryOperator::Eq => "=",
        BinaryOperator::NotEq => "!=",
        BinaryOperator::Lt => "<",
        BinaryOperator::LtEq => "<=",
        BinaryOperator::Gt => ">",
        BinaryOperator::GtEq => ">=",
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Modulo => "%",
        BinaryOperator::And => "AND",
        BinaryOperator::Or => "OR",
    }
}

fn unary_op_to_sql(op: &UnaryOperator) -> &str {
    match op {
        UnaryOperator::Not => "NOT",
        UnaryOperator::Negate => "-",
        UnaryOperator::IsNull => "IS NULL",
        UnaryOperator::IsNotNull => "IS NOT NULL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragment_executor_creation() {
        let query_id = Uuid::new_v4();
        let executor = FragmentExecutor::new(query_id);
        assert_eq!(executor.query_id, query_id);
    }
}
