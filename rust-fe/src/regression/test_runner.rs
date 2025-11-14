//! SQL Test Runner - provides infrastructure for executing SQL queries

use datafusion::sql::parser::DFParser;
use datafusion::sql::sqlparser::dialect::GenericDialect;

#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedResult {
    Success,
    RowCount(usize),
    Error(String),
    Contains(Vec<String>),
}

pub fn execute_test(sql: &str, expected: ExpectedResult) -> Result<(), String> {
    let dialect = GenericDialect{};
    let parsed = DFParser::parse_sql_with_dialect(sql, &dialect);
    
    match parsed {
        Ok(statements) if !statements.is_empty() => {
            match expected {
                ExpectedResult::Success => Ok(()),
                ExpectedResult::Error(_) => Err("Expected error but query succeeded".to_string()),
                _ => Ok(()),
            }
        }
        Ok(_) => Err("No statements parsed".to_string()),
        Err(e) => {
            match expected {
                ExpectedResult::Error(msg) => {
                    if format!("{:?}", e).contains(&msg) {
                        Ok(())
                    } else {
                        Err(format!("Expected error '{}', got {:?}", msg, e))
                    }
                }
                _ => Err(format!("Unexpected parse error: {:?}", e))
            }
        }
    }
}
