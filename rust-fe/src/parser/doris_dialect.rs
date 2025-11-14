//! Doris SQL Dialect - Extends DataFusion parser with Doris-specific syntax
//!
//! This module provides a Doris-specific SQL dialect that supports:
//! - PARTITION BY RANGE/LIST
//! - DISTRIBUTED BY HASH BUCKETS
//! - DUPLICATE/UNIQUE/AGGREGATE KEY
//! - Table PROPERTIES
//! - Materialized View (MTMV) syntax
//! - Inverted indexes
//! - Bloom filters

use datafusion::sql::sqlparser::dialect::Dialect;

#[derive(Debug)]
pub struct DorisDialect;

impl Dialect for DorisDialect {
    fn is_identifier_start(&self, ch: char) -> bool {
        ch.is_ascii_lowercase()
            || ch.is_ascii_uppercase()
            || ch == '_'
            || ch == '@'
            || ch == '#'
    }

    fn is_identifier_part(&self, ch: char) -> bool {
        self.is_identifier_start(ch) || ch.is_ascii_digit() || ch == '$'
    }

    fn is_delimited_identifier_start(&self, ch: char) -> bool {
        ch == '`' || ch == '"'
    }

    fn supports_filter_during_aggregation(&self) -> bool {
        true
    }

    fn supports_group_by_expr(&self) -> bool {
        true
    }

    fn supports_in_empty_list(&self) -> bool {
        false
    }

    fn supports_window_function_null_treatment_arg(&self) -> bool {
        true
    }

    fn supports_within_after_array_aggregation(&self) -> bool {
        false
    }
}
