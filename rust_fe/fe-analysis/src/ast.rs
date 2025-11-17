// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Abstract Syntax Tree definitions

use sqlparser::ast as sql;
use sqlparser::parser::ParserError;

/// Top-level SQL statement
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    CreateTable(CreateTableStatement),
    DropTable(DropTableStatement),
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    CreateDatabase(CreateDatabaseStatement),
    DropDatabase(DropDatabaseStatement),
    Use(UseStatement),
    Show(ShowStatement),
    Set(SetStatement),
}

impl Statement {
    /// Convert from sqlparser AST
    pub fn from_sqlparser(stmt: sql::Statement) -> Result<Self, ParserError> {
        match stmt {
            sql::Statement::CreateTable { .. } => {
                Ok(Statement::CreateTable(CreateTableStatement::from_sqlparser(stmt)?))
            }
            sql::Statement::Drop { .. } => {
                Ok(Statement::DropTable(DropTableStatement::from_sqlparser(stmt)?))
            }
            sql::Statement::Query(_) => {
                Ok(Statement::Select(SelectStatement::from_sqlparser(stmt)?))
            }
            sql::Statement::Insert { .. } => {
                Ok(Statement::Insert(InsertStatement::from_sqlparser(stmt)?))
            }
            sql::Statement::Update { .. } => {
                Ok(Statement::Update(UpdateStatement::from_sqlparser(stmt)?))
            }
            sql::Statement::Delete { .. } => {
                Ok(Statement::Delete(DeleteStatement::from_sqlparser(stmt)?))
            }
            _ => Ok(Statement::Show(ShowStatement { command: "UNKNOWN".to_string() })),
        }
    }
}

/// CREATE TABLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTableStatement {
    pub if_not_exists: bool,
    pub table_name: String,
    pub columns: Vec<ColumnDef>,
    pub keys_type: Option<String>,
    pub distribution: Option<Distribution>,
    pub properties: Vec<(String, String)>,
}

impl CreateTableStatement {
    pub fn from_sqlparser(stmt: sql::Statement) -> Result<Self, ParserError> {
        match stmt {
            sql::Statement::CreateTable(create_table) => {
                let table_name = create_table.name.to_string();
                let columns = create_table.columns.into_iter()
                    .map(ColumnDef::from_sqlparser)
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(CreateTableStatement {
                    if_not_exists: create_table.if_not_exists,
                    table_name,
                    columns,
                    keys_type: None,
                    distribution: None,
                    properties: Vec::new(),
                })
            }
            _ => Err(ParserError::ParserError("Expected CREATE TABLE".to_string())),
        }
    }
}

/// Column definition
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
}

impl ColumnDef {
    fn from_sqlparser(col: sql::ColumnDef) -> Result<Self, ParserError> {
        let mut nullable = true;
        let mut default = None;

        for option in col.options {
            match option.option {
                sql::ColumnOption::NotNull => nullable = false,
                sql::ColumnOption::Default(expr) => default = Some(format!("{}", expr)),
                _ => {}
            }
        }

        Ok(ColumnDef {
            name: col.name.to_string(),
            data_type: col.data_type.to_string(),
            nullable,
            default,
        })
    }
}

/// Distribution clause
#[derive(Debug, Clone, PartialEq)]
pub struct Distribution {
    pub distribution_type: DistributionType,
    pub columns: Vec<String>,
    pub buckets: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DistributionType {
    Hash,
    Random,
}

/// DROP TABLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropTableStatement {
    pub if_exists: bool,
    pub table_name: String,
}

impl DropTableStatement {
    pub fn from_sqlparser(stmt: sql::Statement) -> Result<Self, ParserError> {
        match stmt {
            sql::Statement::Drop { object_type, if_exists, names, .. } => {
                if object_type != sql::ObjectType::Table {
                    return Err(ParserError::ParserError("Expected DROP TABLE".to_string()));
                }
                let table_name = names.first()
                    .ok_or_else(|| ParserError::ParserError("Missing table name".to_string()))?
                    .to_string();

                Ok(DropTableStatement {
                    if_exists,
                    table_name,
                })
            }
            _ => Err(ParserError::ParserError("Expected DROP TABLE".to_string())),
        }
    }
}

/// SELECT statement
#[derive(Debug, Clone, PartialEq)]
pub struct SelectStatement {
    pub query: String, // For now, store as string
}

impl SelectStatement {
    pub fn from_sqlparser(stmt: sql::Statement) -> Result<Self, ParserError> {
        Ok(SelectStatement {
            query: stmt.to_string(),
        })
    }
}

/// INSERT statement
#[derive(Debug, Clone, PartialEq)]
pub struct InsertStatement {
    pub table_name: String,
    pub columns: Vec<String>,
}

impl InsertStatement {
    pub fn from_sqlparser(stmt: sql::Statement) -> Result<Self, ParserError> {
        match stmt {
            sql::Statement::Insert(insert) => {
                Ok(InsertStatement {
                    table_name: insert.table_name.to_string(),
                    columns: insert.columns.iter().map(|c| c.to_string()).collect(),
                })
            }
            _ => Err(ParserError::ParserError("Expected INSERT".to_string())),
        }
    }
}

/// UPDATE statement
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateStatement {
    pub table_name: String,
}

impl UpdateStatement {
    pub fn from_sqlparser(stmt: sql::Statement) -> Result<Self, ParserError> {
        match stmt {
            sql::Statement::Update { table, .. } => {
                Ok(UpdateStatement {
                    table_name: table.relation.to_string(),
                })
            }
            _ => Err(ParserError::ParserError("Expected UPDATE".to_string())),
        }
    }
}

/// DELETE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteStatement {
    pub table_name: String,
}

impl DeleteStatement {
    pub fn from_sqlparser(stmt: sql::Statement) -> Result<Self, ParserError> {
        match stmt {
            sql::Statement::Delete(delete) => {
                // Extract table name - tables field contains ObjectName directly
                let table_name = if !delete.tables.is_empty() {
                    delete.tables.first().unwrap().to_string()
                } else {
                    // Fallback: use from clause
                    match &delete.from {
                        sql::FromTable::WithFromKeyword(tables) => {
                            tables.first()
                                .map(|t| t.relation.to_string())
                                .unwrap_or_default()
                        }
                        sql::FromTable::WithoutKeyword(tables) => {
                            tables.first()
                                .map(|t| t.relation.to_string())
                                .unwrap_or_default()
                        }
                    }
                };

                if table_name.is_empty() {
                    return Err(ParserError::ParserError("Missing table name in DELETE".to_string()));
                }

                Ok(DeleteStatement {
                    table_name,
                })
            }
            _ => Err(ParserError::ParserError("Expected DELETE".to_string())),
        }
    }
}

/// CREATE DATABASE statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateDatabaseStatement {
    pub database_name: String,
}

/// DROP DATABASE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropDatabaseStatement {
    pub database_name: String,
}

/// USE statement
#[derive(Debug, Clone, PartialEq)]
pub struct UseStatement {
    pub database_name: String,
}

/// SHOW statement
#[derive(Debug, Clone, PartialEq)]
pub struct ShowStatement {
    pub command: String,
}

/// SET statement
#[derive(Debug, Clone, PartialEq)]
pub struct SetStatement {
    pub variable: String,
    pub value: String,
}
