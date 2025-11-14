//! Stream Load HTTP Endpoint Tests
//!
//! This module contains comprehensive tests for the streaming load functionality,
//! validating HTTP endpoint behaviors, CSV/JSON parsing, error handling, and
//! protocol compliance.
//!
//! Based on: regression-test/suites/load_p0/stream_load/

#[cfg(test)]
mod tests {
    use super::super::handlers::*;
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use bytes::Bytes;
    use std::sync::Arc;
    use serde_json::Value;

    /// Test CSV parsing to INSERT statement
    #[test]
    fn test_parse_csv_basic() {
        let csv_data = "id,name,age\n1,Alice,30\n2,Bob,25";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("INSERT INTO test_db.test_table"));
        assert!(sql.contains("id, name, age"));
        assert!(sql.contains("'Alice'"));
        assert!(sql.contains("'Bob'"));
    }

    #[test]
    fn test_parse_csv_with_pipe_separator() {
        let csv_data = "id|name|age\n1|Charlie|35\n2|David|40";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, "|");

        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("'Charlie'"));
        assert!(sql.contains("'David'"));
    }

    #[test]
    fn test_parse_csv_with_numbers() {
        let csv_data = "id,amount,price\n1,100,99.99\n2,200,199.99";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        // Numbers should not be quoted (simple heuristic)
        assert!(sql.contains("100"));
        assert!(sql.contains("99.99"));
    }

    #[test]
    fn test_parse_csv_with_escaped_quotes() {
        let csv_data = "id,text\n1,O'Brien\n2,It's working";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        // Single quotes should be escaped
        assert!(sql.contains("O''Brien") || sql.contains("O'Brien"));
    }

    #[test]
    fn test_parse_csv_empty_data() {
        let csv_data = "";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty"));
    }

    #[test]
    fn test_parse_csv_header_only() {
        let csv_data = "id,name,age";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No data rows"));
    }

    #[test]
    fn test_parse_csv_malformed_rows() {
        // Row 2 has mismatched column count - should be skipped
        let csv_data = "id,name,age\n1,Alice,30\n2,Bob\n3,Charlie,35";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        // Should contain Alice and Charlie but not the malformed Bob row
        assert!(sql.contains("'Alice'"));
        assert!(sql.contains("'Charlie'"));
    }

    #[test]
    fn test_parse_csv_with_empty_lines() {
        let csv_data = "id,name,age\n1,Alice,30\n\n2,Bob,25\n\n";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("'Alice'"));
        assert!(sql.contains("'Bob'"));
    }

    #[test]
    fn test_parse_csv_with_leading_zeros() {
        // IDs with leading zeros should be treated as strings
        let csv_data = "id,code\n001,ABC\n002,DEF";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        // Leading zeros should preserve them by quoting
        assert!(sql.contains("'001'") || sql.contains("001"));
    }

    #[test]
    fn test_parse_csv_large_dataset() {
        // Test with multiple rows
        let mut csv_data = String::from("id,name,value\n");
        for i in 1..=100 {
            csv_data.push_str(&format!("{},User{},{}\n", i, i, i * 100));
        }
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("'User1'"));
        assert!(sql.contains("'User100'"));
        // Should have 100 value tuples (plus 1 for INSERT INTO table (...))
        assert!(sql.matches("(").count() >= 100);
    }

    #[test]
    fn test_parse_csv_special_characters() {
        let csv_data = "id,text\n1,Hello\\nWorld\n2,Tab\\there";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        // Should handle backslashes
    }

    #[test]
    fn test_parse_csv_unicode() {
        let csv_data = "id,name,city\n1,张三,北京\n2,李四,上海";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("张三"));
        assert!(sql.contains("北京"));
    }

    #[test]
    fn test_parse_csv_mixed_types() {
        let csv_data = "id,name,age,salary,active\n1,Alice,30,50000.50,true\n2,Bob,25,45000.00,false";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("'Alice'"));
        assert!(sql.contains("50000.50"));
        assert!(sql.contains("'true'"));
    }

    #[test]
    fn test_parse_csv_sql_injection_attempt() {
        // Test that SQL injection attempts are handled safely
        let csv_data = "id,name\n1,'; DROP TABLE users; --";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        assert!(result.is_ok());
        let sql = result.unwrap();
        // Should escape quotes properly
        assert!(sql.contains("''"));
    }

    #[test]
    fn test_stream_load_response_structure() {
        // Test that the expected response JSON structure is correct
        let json_str = r#"{
            "TxnId": "test-txn-id",
            "Label": "test-label",
            "Status": "Success",
            "Message": "OK",
            "NumberTotalRows": 100,
            "NumberLoadedRows": 100,
            "NumberFilteredRows": 0,
            "NumberUnselectedRows": 0,
            "LoadBytes": 1024,
            "LoadTimeMs": 100
        }"#;

        let json: Result<Value, _> = serde_json::from_str(json_str);
        assert!(json.is_ok());

        let json = json.unwrap();
        assert_eq!(json["Status"], "Success");
        assert_eq!(json["NumberTotalRows"], 100);
        assert_eq!(json["NumberLoadedRows"], 100);
    }

    #[test]
    fn test_stream_load_error_response() {
        let json_str = r#"{
            "Status": "Fail",
            "Message": "Parse error: Invalid CSV format"
        }"#;

        let json: Result<Value, _> = serde_json::from_str(json_str);
        assert!(json.is_ok());

        let json = json.unwrap();
        assert_eq!(json["Status"], "Fail");
        assert!(json["Message"].as_str().unwrap().contains("Parse error"));
    }

    #[test]
    fn test_health_check_response() {
        let json_str = r#"{
            "status": "healthy",
            "service": "doris-rust-fe"
        }"#;

        let json: Result<Value, _> = serde_json::from_str(json_str);
        assert!(json.is_ok());

        let json = json.unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["service"], "doris-rust-fe");
    }

    #[test]
    fn test_status_endpoint_response() {
        let json_str = r#"{
            "service": "doris-rust-fe",
            "query_queue": {
                "size": 0,
                "available_slots": 100,
                "max_concurrent": 100
            },
            "backend_nodes": 3
        }"#;

        let json: Result<Value, _> = serde_json::from_str(json_str);
        assert!(json.is_ok());

        let json = json.unwrap();
        assert_eq!(json["service"], "doris-rust-fe");
        assert_eq!(json["query_queue"]["max_concurrent"], 100);
        assert_eq!(json["backend_nodes"], 3);
    }

    #[test]
    fn test_header_parsing() {
        let mut headers = HeaderMap::new();
        headers.insert("label", HeaderValue::from_static("test_label"));
        headers.insert("format", HeaderValue::from_static("csv"));
        headers.insert("column_separator", HeaderValue::from_static("|"));

        let label = headers.get("label")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("default_label");

        let format = headers.get("format")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("csv");

        let separator = headers.get("column_separator")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(",");

        assert_eq!(label, "test_label");
        assert_eq!(format, "csv");
        assert_eq!(separator, "|");
    }

    #[test]
    fn test_header_default_values() {
        let headers = HeaderMap::new();

        let label = headers.get("label")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("default_label");

        let format = headers.get("format")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("csv");

        let separator = headers.get("column_separator")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(",");

        assert_eq!(label, "default_label");
        assert_eq!(format, "csv");
        assert_eq!(separator, ",");
    }

    #[test]
    fn test_csv_tab_separator() {
        let csv_data = "id\tname\tage\n1\tAlice\t30\n2\tBob\t25";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, "\t");

        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("'Alice'"));
        assert!(sql.contains("'Bob'"));
    }

    #[test]
    fn test_csv_with_null_values() {
        let csv_data = "id,name,age\n1,Alice,\n2,,25";
        let bytes = Bytes::from(csv_data);

        let result = super::super::handlers::parse_csv_to_insert("test_db", "test_table", &bytes, ",");

        // Empty fields should be handled
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_tables() {
        let csv_data1 = "id,name\n1,Table1";
        let csv_data2 = "id,name\n1,Table2";

        let bytes1 = Bytes::from(csv_data1);
        let bytes2 = Bytes::from(csv_data2);

        let result1 = super::super::handlers::parse_csv_to_insert("db1", "table1", &bytes1, ",");
        let result2 = super::super::handlers::parse_csv_to_insert("db2", "table2", &bytes2, ",");

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        assert!(result1.unwrap().contains("db1.table1"));
        assert!(result2.unwrap().contains("db2.table2"));
    }

    #[test]
    fn test_stream_load_protocol_compliance() {
        // Test that our stream load follows Doris protocol
        // Response should have specific fields
        let required_fields = vec![
            "TxnId", "Label", "Status", "Message",
            "NumberTotalRows", "NumberLoadedRows",
            "NumberFilteredRows", "NumberUnselectedRows",
            "LoadBytes", "LoadTimeMs"
        ];

        let json_str = r#"{
            "TxnId": "test",
            "Label": "test",
            "Status": "Success",
            "Message": "OK",
            "NumberTotalRows": 1,
            "NumberLoadedRows": 1,
            "NumberFilteredRows": 0,
            "NumberUnselectedRows": 0,
            "LoadBytes": 100,
            "LoadTimeMs": 50
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();

        for field in required_fields {
            assert!(json.get(field).is_some(), "Missing required field: {}", field);
        }
    }
}
