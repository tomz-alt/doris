// Doris Plan Fragment structures
// These represent the plan fragments that will be sent to BE for execution

use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// A complete query plan split into fragments for distributed execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub query_id: Uuid,
    pub fragments: Vec<PlanFragment>,
}

/// A single fragment of a query plan that can be executed independently
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFragment {
    pub fragment_id: u32,
    pub root_node: PlanNode,
    pub output_exprs: Vec<Expr>,
    pub partition_info: Option<PartitionInfo>,
}

/// Node types in the Doris execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanNode {
    /// Scan OLAP table (tablets)
    OlapScan {
        table_name: String,
        columns: Vec<String>,
        predicates: Vec<Expr>,
        tablet_ids: Vec<i64>,
    },

    /// Filter/Selection
    Select {
        child: Box<PlanNode>,
        predicates: Vec<Expr>,
    },

    /// Projection
    Project {
        child: Box<PlanNode>,
        exprs: Vec<Expr>,
    },

    /// Aggregation
    Aggregation {
        child: Box<PlanNode>,
        group_by_exprs: Vec<Expr>,
        agg_functions: Vec<AggregateFunction>,
        is_merge: bool, // true for final merge, false for local agg
    },

    /// Hash Join
    HashJoin {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        join_predicates: Vec<Expr>,
    },

    /// Sort
    Sort {
        child: Box<PlanNode>,
        order_by: Vec<OrderByExpr>,
        limit: Option<usize>,
    },

    /// TopN (combined sort + limit)
    TopN {
        child: Box<PlanNode>,
        order_by: Vec<OrderByExpr>,
        limit: usize,
        offset: usize,
    },

    /// Exchange (data shuffle between fragments)
    Exchange {
        child: Box<PlanNode>,
        exchange_type: ExchangeType,
    },

    /// Union
    Union {
        children: Vec<PlanNode>,
    },
}

/// Expression types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Column {
        name: String,
        data_type: DataType,
    },
    Literal {
        value: LiteralValue,
        data_type: DataType,
    },
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },
    Cast {
        expr: Box<Expr>,
        target_type: DataType,
    },
    Function {
        name: String,
        args: Vec<Expr>,
    },
}

/// Aggregate functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregateFunction {
    Count { expr: Option<Expr>, distinct: bool },
    Sum { expr: Expr, distinct: bool },
    Avg { expr: Expr, distinct: bool },
    Min { expr: Expr },
    Max { expr: Expr },
}

/// Join types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Semi,
    Anti,
}

/// Exchange types (how data is shuffled between fragments)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExchangeType {
    /// Broadcast to all nodes
    Broadcast,
    /// Hash partition by keys
    HashPartition { partition_keys: Vec<Expr> },
    /// Random distribution
    Random,
    /// Gather all data to single node
    Gather,
}

/// Order by expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByExpr {
    pub expr: Expr,
    pub ascending: bool,
    pub nulls_first: bool,
}

/// Partition information for a fragment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub partition_type: PartitionType,
    pub num_partitions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionType {
    Random,
    Hash { keys: Vec<String> },
    Range { keys: Vec<String> },
}

/// Data types matching Doris types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Boolean,
    TinyInt,
    SmallInt,
    Int,
    BigInt,
    Float,
    Double,
    Decimal { precision: u8, scale: u8 },
    Char { length: u32 },
    Varchar { length: u32 },
    String,
    Date,
    DateTime,
    Timestamp,
}

/// Literal values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralValue {
    Null,
    Boolean(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
    Date(String),      // ISO format
    DateTime(String),  // ISO format
}

/// Binary operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // Logical
    And,
    Or,
}

/// Unary operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Negate,
    IsNull,
    IsNotNull,
}

impl PlanFragment {
    pub fn new(fragment_id: u32, root_node: PlanNode) -> Self {
        Self {
            fragment_id,
            root_node,
            output_exprs: vec![],
            partition_info: None,
        }
    }

    pub fn with_output_exprs(mut self, exprs: Vec<Expr>) -> Self {
        self.output_exprs = exprs;
        self
    }

    pub fn with_partition_info(mut self, info: PartitionInfo) -> Self {
        self.partition_info = Some(info);
        self
    }
}

impl QueryPlan {
    pub fn new(query_id: Uuid) -> Self {
        Self {
            query_id,
            fragments: vec![],
        }
    }

    pub fn add_fragment(&mut self, fragment: PlanFragment) {
        self.fragments.push(fragment);
    }
}
