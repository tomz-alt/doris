// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! MySQL Result Set Encoding
//!
//! Encodes query results into MySQL protocol format:
//! 1. Column count packet
//! 2. Column definition packets (one per column)
//! 3. EOF packet (if not CLIENT_DEPRECATE_EOF)
//! 4. Row data packets
//! 5. EOF or OK packet

use crate::constants::*;
use crate::packet::{Packet, write_lenenc_string, write_lenenc_int};
use byteorder::{LittleEndian, WriteBytesExt};

/// Column definition
#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub catalog: String,
    pub schema: String,
    pub table: String,
    pub org_table: String,
    pub name: String,
    pub org_name: String,
    pub character_set: u16,
    pub column_length: u32,
    pub column_type: u8,
    pub flags: u16,
    pub decimals: u8,
}

impl ColumnDefinition {
    pub fn new(name: String, column_type: u8) -> Self {
        Self {
            catalog: "def".to_string(),
            schema: "".to_string(),
            table: "".to_string(),
            org_table: "".to_string(),
            name: name.clone(),
            org_name: name,
            character_set: 0x21, // UTF-8
            column_length: 255,
            column_type,
            flags: 0,
            decimals: 0,
        }
    }

    /// Encode to packet payload (Protocol 4.1)
    pub fn to_packet(&self, sequence_id: u8) -> Packet {
        let mut payload = Vec::new();

        // Catalog (length-encoded string)
        write_lenenc_string(&mut payload, self.catalog.as_bytes());

        // Schema (length-encoded string)
        write_lenenc_string(&mut payload, self.schema.as_bytes());

        // Table (length-encoded string)
        write_lenenc_string(&mut payload, self.table.as_bytes());

        // Org table (length-encoded string)
        write_lenenc_string(&mut payload, self.org_table.as_bytes());

        // Name (length-encoded string)
        write_lenenc_string(&mut payload, self.name.as_bytes());

        // Org name (length-encoded string)
        write_lenenc_string(&mut payload, self.org_name.as_bytes());

        // Length of fixed-length fields (always 0x0c = 12)
        write_lenenc_int(&mut payload, 0x0c);

        // Character set (2 bytes)
        payload.write_u16::<LittleEndian>(self.character_set).unwrap();

        // Column length (4 bytes)
        payload.write_u32::<LittleEndian>(self.column_length).unwrap();

        // Column type (1 byte)
        payload.push(self.column_type);

        // Flags (2 bytes)
        payload.write_u16::<LittleEndian>(self.flags).unwrap();

        // Decimals (1 byte)
        payload.push(self.decimals);

        // Filler (2 bytes of 0x00)
        payload.write_u16::<LittleEndian>(0).unwrap();

        Packet::new(sequence_id, payload)
    }
}

/// Text protocol row data
#[derive(Debug, Clone)]
pub struct TextResultRow {
    pub values: Vec<Option<Vec<u8>>>,
}

impl TextResultRow {
    pub fn new(values: Vec<Option<Vec<u8>>>) -> Self {
        Self { values }
    }

    /// Encode to packet payload
    pub fn to_packet(&self, sequence_id: u8) -> Packet {
        let mut payload = Vec::new();

        for value in &self.values {
            match value {
                Some(data) => write_lenenc_string(&mut payload, data),
                None => payload.push(0xFB), // NULL marker
            }
        }

        Packet::new(sequence_id, payload)
    }
}

/// Result set builder
pub struct ResultSet {
    columns: Vec<ColumnDefinition>,
    rows: Vec<TextResultRow>,
}

impl ResultSet {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }

    pub fn add_column(&mut self, column: ColumnDefinition) {
        self.columns.push(column);
    }

    pub fn add_row(&mut self, row: TextResultRow) {
        self.rows.push(row);
    }

    /// Encode entire result set to packets
    pub fn to_packets(&self, mut sequence_id: u8, deprecate_eof: bool) -> Vec<Packet> {
        let mut packets = Vec::new();

        // 1. Column count packet
        let mut col_count_payload = Vec::new();
        write_lenenc_int(&mut col_count_payload, self.columns.len() as u64);
        packets.push(Packet::new(sequence_id, col_count_payload));
        sequence_id = sequence_id.wrapping_add(1);

        // 2. Column definition packets
        for column in &self.columns {
            packets.push(column.to_packet(sequence_id));
            sequence_id = sequence_id.wrapping_add(1);
        }

        // 3. EOF packet after column definitions (if not deprecate_eof and has columns)
        if !deprecate_eof && !self.columns.is_empty() {
            packets.push(Packet::eof(0, SERVER_STATUS_AUTOCOMMIT, sequence_id));
            sequence_id = sequence_id.wrapping_add(1);
        }

        // 4. Row data packets
        for row in &self.rows {
            packets.push(row.to_packet(sequence_id));
            sequence_id = sequence_id.wrapping_add(1);
        }

        // 5. Final EOF or OK packet
        if deprecate_eof {
            // Use OK packet with SERVER_STATUS_LAST_ROW_SENT
            packets.push(Packet::ok(
                self.rows.len() as u64,
                0,
                SERVER_STATUS_AUTOCOMMIT,
                0,
                sequence_id,
            ));
        } else {
            packets.push(Packet::eof(0, SERVER_STATUS_AUTOCOMMIT, sequence_id));
        }

        packets
    }
}

impl Default for ResultSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to convert Doris DataType to MySQL column type
pub fn datatype_to_mysql_type(dt: &str) -> u8 {
    let upper = dt.to_uppercase();
    match upper.as_str() {
        s if s.starts_with("DATETIME") => MYSQL_TYPE_DATETIME,
        s if s.starts_with("DATE") => MYSQL_TYPE_DATE,
        s if s.starts_with("TIMESTAMP") => MYSQL_TYPE_TIMESTAMP,
        s if s.starts_with("TINYINT") => MYSQL_TYPE_TINY,
        s if s.starts_with("SMALLINT") => MYSQL_TYPE_SHORT,
        s if s.starts_with("INT") || s.starts_with("INTEGER") => MYSQL_TYPE_LONG,
        s if s.starts_with("BIGINT") => MYSQL_TYPE_LONGLONG,
        s if s.starts_with("FLOAT") => MYSQL_TYPE_FLOAT,
        s if s.starts_with("DOUBLE") => MYSQL_TYPE_DOUBLE,
        s if s.starts_with("DECIMAL") => MYSQL_TYPE_NEWDECIMAL,
        s if s.starts_with("CHAR") => MYSQL_TYPE_STRING,
        s if s.starts_with("VARCHAR") => MYSQL_TYPE_VARCHAR,
        s if s.starts_with("TEXT") || s.starts_with("STRING") => MYSQL_TYPE_VAR_STRING,
        s if s.starts_with("BLOB") => MYSQL_TYPE_BLOB,
        _ => MYSQL_TYPE_VAR_STRING, // Default to VAR_STRING
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_definition() {
        let col = ColumnDefinition::new("user_id".to_string(), MYSQL_TYPE_LONG);
        assert_eq!(col.name, "user_id");
        assert_eq!(col.column_type, MYSQL_TYPE_LONG);
        assert_eq!(col.catalog, "def");
    }

    #[test]
    fn test_column_definition_packet() {
        let col = ColumnDefinition::new("test_col".to_string(), MYSQL_TYPE_VARCHAR);
        let packet = col.to_packet(1);

        assert_eq!(packet.sequence_id, 1);
        assert!(!packet.payload.is_empty());

        // Verify it contains the column name
        let payload_str = String::from_utf8_lossy(&packet.payload);
        assert!(payload_str.contains("test_col"));
    }

    #[test]
    fn test_text_result_row() {
        let row = TextResultRow::new(vec![
            Some(b"1".to_vec()),
            Some(b"John".to_vec()),
            None, // NULL value
        ]);

        let packet = row.to_packet(5);
        assert_eq!(packet.sequence_id, 5);

        // Check NULL marker is present
        assert!(packet.payload.contains(&0xFB));
    }

    #[test]
    fn test_result_set_simple() {
        let mut rs = ResultSet::new();

        // Add columns
        rs.add_column(ColumnDefinition::new("id".to_string(), MYSQL_TYPE_LONG));
        rs.add_column(ColumnDefinition::new("name".to_string(), MYSQL_TYPE_VARCHAR));

        // Add rows
        rs.add_row(TextResultRow::new(vec![
            Some(b"1".to_vec()),
            Some(b"Alice".to_vec()),
        ]));
        rs.add_row(TextResultRow::new(vec![
            Some(b"2".to_vec()),
            Some(b"Bob".to_vec()),
        ]));

        // Generate packets
        let packets = rs.to_packets(1, false);

        // Should have: 1 col count + 2 col defs + 1 EOF + 2 rows + 1 EOF = 7 packets
        assert_eq!(packets.len(), 7);

        // First packet is column count
        assert_eq!(packets[0].sequence_id, 1);
        assert_eq!(packets[0].payload[0], 2); // 2 columns

        // Last packet is EOF
        assert_eq!(packets[6].payload[0], 0xFE);
    }

    #[test]
    fn test_result_set_with_deprecate_eof() {
        let mut rs = ResultSet::new();
        rs.add_column(ColumnDefinition::new("id".to_string(), MYSQL_TYPE_LONG));
        rs.add_row(TextResultRow::new(vec![Some(b"1".to_vec())]));

        let packets = rs.to_packets(1, true);

        // Should have: 1 col count + 1 col def + 1 row + 1 OK = 4 packets
        assert_eq!(packets.len(), 4);

        // Last packet is OK (not EOF)
        assert_eq!(packets[3].payload[0], 0x00);
    }

    #[test]
    fn test_datatype_conversion() {
        assert_eq!(datatype_to_mysql_type("INT"), MYSQL_TYPE_LONG);
        assert_eq!(datatype_to_mysql_type("INTEGER"), MYSQL_TYPE_LONG);
        assert_eq!(datatype_to_mysql_type("BIGINT"), MYSQL_TYPE_LONGLONG);
        assert_eq!(datatype_to_mysql_type("VARCHAR(255)"), MYSQL_TYPE_VARCHAR);
        assert_eq!(datatype_to_mysql_type("DECIMAL(10,2)"), MYSQL_TYPE_NEWDECIMAL);
        assert_eq!(datatype_to_mysql_type("DATE"), MYSQL_TYPE_DATE);
        assert_eq!(datatype_to_mysql_type("DATETIME"), MYSQL_TYPE_DATETIME);
        assert_eq!(datatype_to_mysql_type("UNKNOWN"), MYSQL_TYPE_VAR_STRING);
    }

    #[test]
    fn test_empty_result_set() {
        let rs = ResultSet::new();
        let packets = rs.to_packets(1, false);

        // Should have: 1 col count (0) + 1 EOF = 2 packets
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].payload[0], 0); // 0 columns
    }
}
