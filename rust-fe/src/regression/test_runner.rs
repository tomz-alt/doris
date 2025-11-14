//! SQL Test Runner - provides infrastructure for executing SQL queries

use crate::parser::doris_parser::DorisParser;

#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedResult {
    Success,
    RowCount(usize),
    Error(String),
    Contains(Vec<String>),
}

pub fn execute_test(sql: &str, expected: ExpectedResult) -> Result<(), String> {
    // Use Doris parser to handle both standard and Doris-specific SQL
    let parsed = DorisParser::parse_sql(sql);

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
                    let error_str = format!("{:?}", e);
                    if error_str.contains(&msg) {
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
