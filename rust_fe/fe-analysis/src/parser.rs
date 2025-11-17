// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! SQL Parser using sqlparser-rs with Doris extensions

use sqlparser::parser::{Parser, ParserError};
use sqlparser::dialect::GenericDialect;
use sqlparser::tokenizer::Tokenizer;
use crate::ast::*;

pub struct DorisParser;

impl DorisParser {
    /// Parse a SQL statement
    pub fn parse(sql: &str) -> Result<Vec<Statement>, ParserError> {
        let dialect = GenericDialect {};
        let tokens = Tokenizer::new(&dialect, sql).tokenize()?;

        let mut parser = Parser::new(&dialect)
            .with_tokens(tokens);

        let ast = parser.parse_statements()?;

        // Convert sqlparser AST to our custom AST
        let mut statements = Vec::new();
        for stmt in ast {
            statements.push(Statement::from_sqlparser(stmt)?);
        }

        Ok(statements)
    }

    /// Parse a single SQL statement
    pub fn parse_one(sql: &str) -> Result<Statement, ParserError> {
        let mut stmts = Self::parse(sql)?;
        if stmts.len() != 1 {
            return Err(ParserError::ParserError(
                format!("Expected exactly one statement, got {}", stmts.len())
            ));
        }
        Ok(stmts.remove(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let sql = "SELECT * FROM users";
        let result = DorisParser::parse_one(sql);
        assert!(result.is_ok(), "Failed to parse simple SELECT: {:?}", result.err());
    }

    #[test]
    fn test_parse_create_table() {
        let sql = r#"
            CREATE TABLE users (
                id INT NOT NULL,
                name VARCHAR(50),
                email VARCHAR(100)
            )
        "#;
        let result = DorisParser::parse_one(sql);
        assert!(result.is_ok(), "Failed to parse CREATE TABLE: {:?}", result.err());
    }

    #[test]
    fn test_parse_tpch_lineitem() {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS lineitem (
                L_ORDERKEY INTEGER NOT NULL,
                L_PARTKEY INTEGER NOT NULL,
                L_SUPPKEY INTEGER NOT NULL,
                L_LINENUMBER INTEGER NOT NULL,
                L_QUANTITY DECIMAL(15,2) NOT NULL,
                L_EXTENDEDPRICE DECIMAL(15,2) NOT NULL,
                L_DISCOUNT DECIMAL(15,2) NOT NULL,
                L_TAX DECIMAL(15,2) NOT NULL,
                L_RETURNFLAG CHAR(1) NOT NULL,
                L_LINESTATUS CHAR(1) NOT NULL,
                L_SHIPDATE DATE NOT NULL,
                L_COMMITDATE DATE NOT NULL,
                L_RECEIPTDATE DATE NOT NULL,
                L_SHIPINSTRUCT CHAR(25) NOT NULL,
                L_SHIPMODE CHAR(10) NOT NULL,
                L_COMMENT VARCHAR(44) NOT NULL
            )
        "#;
        let result = DorisParser::parse_one(sql);
        assert!(result.is_ok(), "Failed to parse TPC-H lineitem table: {:?}", result.err());
    }

    #[test]
    fn test_parse_tpch_q1() {
        let sql = r#"
            select
                l_returnflag,
                l_linestatus,
                sum(l_quantity) as sum_qty,
                sum(l_extendedprice) as sum_base_price,
                sum(l_extendedprice * (1 - l_discount)) as sum_disc_price,
                sum(l_extendedprice * (1 - l_discount) * (1 + l_tax)) as sum_charge,
                avg(l_quantity) as avg_qty,
                avg(l_extendedprice) as avg_price,
                avg(l_discount) as avg_disc,
                count(*) as count_order
            from
                lineitem
            where
                l_shipdate <= date '1998-12-01' - interval '90' day
            group by
                l_returnflag,
                l_linestatus
            order by
                l_returnflag,
                l_linestatus
        "#;
        let result = DorisParser::parse_one(sql);
        assert!(result.is_ok(), "Failed to parse TPC-H Q1: {:?}", result.err());
    }

    #[test]
    fn test_parse_multiple_statements() {
        let sql = "SELECT 1; SELECT 2; SELECT 3;";
        let result = DorisParser::parse(sql);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_parse_insert() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
        let result = DorisParser::parse_one(sql);
        assert!(result.is_ok(), "Failed to parse INSERT: {:?}", result.err());
    }

    #[test]
    fn test_parse_delete() {
        let sql = "DELETE FROM users WHERE id = 1";
        let result = DorisParser::parse_one(sql);
        assert!(result.is_ok(), "Failed to parse DELETE: {:?}", result.err());
    }

    #[test]
    fn test_parse_update() {
        let sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
        let result = DorisParser::parse_one(sql);
        assert!(result.is_ok(), "Failed to parse UPDATE: {:?}", result.err());
    }

    #[test]
    fn test_parse_drop_table() {
        let sql = "DROP TABLE users";
        let result = DorisParser::parse_one(sql);
        assert!(result.is_ok(), "Failed to parse DROP TABLE: {:?}", result.err());
    }
}
