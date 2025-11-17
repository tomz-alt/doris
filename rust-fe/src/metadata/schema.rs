use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::types::DataType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub comment: Option<String>,
    pub auto_increment: bool,
    pub primary_key: bool,
}

impl ColumnDef {
    pub fn new(name: String, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            nullable: true,
            default_value: None,
            comment: None,
            auto_increment: false,
            primary_key: false,
        }
    }

    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self
    }

    pub fn default(mut self, value: String) -> Self {
        self.default_value = Some(value);
        self
    }

    pub fn comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }
}

// Alias for easier usage
pub type Column = ColumnDef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub primary_keys: Vec<String>,
    pub partition_keys: Vec<String>,
    pub distribution_keys: Vec<String>,
    pub properties: HashMap<String, String>,
    pub comment: Option<String>,
}

impl Table {
    pub fn new(name: String) -> Self {
        Self {
            name,
            columns: Vec::new(),
            primary_keys: Vec::new(),
            partition_keys: Vec::new(),
            distribution_keys: Vec::new(),
            properties: HashMap::new(),
            comment: None,
        }
    }

    pub fn add_column(mut self, column: ColumnDef) -> Self {
        if column.primary_key && !self.primary_keys.contains(&column.name) {
            self.primary_keys.push(column.name.clone());
        }
        self.columns.push(column);
        self
    }

    pub fn with_columns(mut self, columns: Vec<ColumnDef>) -> Self {
        for column in columns {
            self = self.add_column(column);
        }
        self
    }

    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    pub fn get_column(&self, name: &str) -> Option<&ColumnDef> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    pub name: String,
    pub tables: HashMap<String, Table>,
    pub comment: Option<String>,
}

impl Database {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tables: HashMap::new(),
            comment: None,
        }
    }

    pub fn add_table(&mut self, table: Table) {
        self.tables.insert(table.name.clone(), table);
    }

    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }

    pub fn get_table_mut(&mut self, name: &str) -> Option<&mut Table> {
        self.tables.get_mut(name)
    }

    pub fn drop_table(&mut self, name: &str) -> Option<Table> {
        self.tables.remove(name)
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }
}

// TPC-H schema helper
impl Database {
    /// Create TPC-H database with all required tables
    pub fn create_tpch_schema() -> Self {
        let mut db = Database::new("tpch_sf1".to_string());
        db.comment = Some("TPC-H Benchmark Database (SF1)".to_string());

        // Create NATION table
        db.add_table(
            Table::new("nation".to_string())
                .with_columns(vec![
                    ColumnDef::new("n_nationkey".to_string(), DataType::Int).not_null().primary_key(),
                    ColumnDef::new("n_name".to_string(), DataType::Varchar { length: 25 }).not_null(),
                    ColumnDef::new("n_regionkey".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("n_comment".to_string(), DataType::Varchar { length: 152 }),
                ])
                .with_comment("Nations table".to_string())
        );

        // Create REGION table
        db.add_table(
            Table::new("region".to_string())
                .with_columns(vec![
                    ColumnDef::new("r_regionkey".to_string(), DataType::Int).not_null().primary_key(),
                    ColumnDef::new("r_name".to_string(), DataType::Varchar { length: 25 }).not_null(),
                    ColumnDef::new("r_comment".to_string(), DataType::Varchar { length: 152 }),
                ])
                .with_comment("Regions table".to_string())
        );

        // Create PART table
        db.add_table(
            Table::new("part".to_string())
                .with_columns(vec![
                    ColumnDef::new("p_partkey".to_string(), DataType::Int).not_null().primary_key(),
                    ColumnDef::new("p_name".to_string(), DataType::Varchar { length: 55 }).not_null(),
                    ColumnDef::new("p_mfgr".to_string(), DataType::Varchar { length: 25 }).not_null(),
                    ColumnDef::new("p_brand".to_string(), DataType::Varchar { length: 10 }).not_null(),
                    ColumnDef::new("p_type".to_string(), DataType::Varchar { length: 25 }).not_null(),
                    ColumnDef::new("p_size".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("p_container".to_string(), DataType::Varchar { length: 10 }).not_null(),
                    ColumnDef::new("p_retailprice".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("p_comment".to_string(), DataType::Varchar { length: 23 }).not_null(),
                ])
                .with_comment("Parts table".to_string())
        );

        // Create SUPPLIER table
        db.add_table(
            Table::new("supplier".to_string())
                .with_columns(vec![
                    ColumnDef::new("s_suppkey".to_string(), DataType::Int).not_null().primary_key(),
                    ColumnDef::new("s_name".to_string(), DataType::Varchar { length: 25 }).not_null(),
                    ColumnDef::new("s_address".to_string(), DataType::Varchar { length: 40 }).not_null(),
                    ColumnDef::new("s_nationkey".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("s_phone".to_string(), DataType::Varchar { length: 15 }).not_null(),
                    ColumnDef::new("s_acctbal".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("s_comment".to_string(), DataType::Varchar { length: 101 }).not_null(),
                ])
                .with_comment("Suppliers table".to_string())
        );

        // Create PARTSUPP table
        db.add_table(
            Table::new("partsupp".to_string())
                .with_columns(vec![
                    ColumnDef::new("ps_partkey".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("ps_suppkey".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("ps_availqty".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("ps_supplycost".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("ps_comment".to_string(), DataType::Varchar { length: 199 }).not_null(),
                ])
                .with_comment("Part-Supplier relationship table".to_string())
        );

        // Create CUSTOMER table
        db.add_table(
            Table::new("customer".to_string())
                .with_columns(vec![
                    ColumnDef::new("c_custkey".to_string(), DataType::Int).not_null().primary_key(),
                    ColumnDef::new("c_name".to_string(), DataType::Varchar { length: 25 }).not_null(),
                    ColumnDef::new("c_address".to_string(), DataType::Varchar { length: 40 }).not_null(),
                    ColumnDef::new("c_nationkey".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("c_phone".to_string(), DataType::Varchar { length: 15 }).not_null(),
                    ColumnDef::new("c_acctbal".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("c_mktsegment".to_string(), DataType::Varchar { length: 10 }).not_null(),
                    ColumnDef::new("c_comment".to_string(), DataType::Varchar { length: 117 }).not_null(),
                ])
                .with_comment("Customers table".to_string())
        );

        // Create ORDERS table
        db.add_table(
            Table::new("orders".to_string())
                .with_columns(vec![
                    ColumnDef::new("o_orderkey".to_string(), DataType::Int).not_null().primary_key(),
                    ColumnDef::new("o_custkey".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("o_orderstatus".to_string(), DataType::Varchar { length: 1 }).not_null(),
                    ColumnDef::new("o_totalprice".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("o_orderdate".to_string(), DataType::Date).not_null(),
                    ColumnDef::new("o_orderpriority".to_string(), DataType::Varchar { length: 15 }).not_null(),
                    ColumnDef::new("o_clerk".to_string(), DataType::Varchar { length: 15 }).not_null(),
                    ColumnDef::new("o_shippriority".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("o_comment".to_string(), DataType::Varchar { length: 79 }).not_null(),
                ])
                .with_comment("Orders table".to_string())
        );

        // Create LINEITEM table
        db.add_table(
            Table::new("lineitem".to_string())
                .with_columns(vec![
                    ColumnDef::new("l_orderkey".to_string(), DataType::BigInt).not_null(),
                    ColumnDef::new("l_partkey".to_string(), DataType::BigInt).not_null(),
                    ColumnDef::new("l_suppkey".to_string(), DataType::BigInt).not_null(),
                    ColumnDef::new("l_linenumber".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("l_quantity".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("l_extendedprice".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("l_discount".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("l_tax".to_string(), DataType::Decimal { precision: 15, scale: 2 }).not_null(),
                    ColumnDef::new("l_returnflag".to_string(), DataType::Varchar { length: 1 }).not_null(),
                    ColumnDef::new("l_linestatus".to_string(), DataType::Varchar { length: 1 }).not_null(),
                    ColumnDef::new("l_shipdate".to_string(), DataType::Date).not_null(),
                    ColumnDef::new("l_commitdate".to_string(), DataType::Date).not_null(),
                    ColumnDef::new("l_receiptdate".to_string(), DataType::Date).not_null(),
                    ColumnDef::new("l_shipinstruct".to_string(), DataType::Varchar { length: 25 }).not_null(),
                    ColumnDef::new("l_shipmode".to_string(), DataType::Varchar { length: 10 }).not_null(),
                    ColumnDef::new("l_comment".to_string(), DataType::Varchar { length: 44 }).not_null(),
                ])
                .with_comment("Line items table".to_string())
        );

        db
    }
}
