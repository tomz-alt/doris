// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! MySQL Protocol Constants

// MySQL Protocol Version
pub const PROTOCOL_VERSION: u8 = 10;

// Server version string
pub const SERVER_VERSION: &str = "5.7.99-Doris-Rust-1.0";

// Capability flags
pub const CLIENT_LONG_PASSWORD: u32 = 0x00000001;
pub const CLIENT_FOUND_ROWS: u32 = 0x00000002;
pub const CLIENT_LONG_FLAG: u32 = 0x00000004;
pub const CLIENT_CONNECT_WITH_DB: u32 = 0x00000008;
pub const CLIENT_NO_SCHEMA: u32 = 0x00000010;
pub const CLIENT_COMPRESS: u32 = 0x00000020;
pub const CLIENT_ODBC: u32 = 0x00000040;
pub const CLIENT_LOCAL_FILES: u32 = 0x00000080;
pub const CLIENT_IGNORE_SPACE: u32 = 0x00000100;
pub const CLIENT_PROTOCOL_41: u32 = 0x00000200;
pub const CLIENT_INTERACTIVE: u32 = 0x00000400;
pub const CLIENT_SSL: u32 = 0x00000800;
pub const CLIENT_IGNORE_SIGPIPE: u32 = 0x00001000;
pub const CLIENT_TRANSACTIONS: u32 = 0x00002000;
pub const CLIENT_RESERVED: u32 = 0x00004000;
pub const CLIENT_SECURE_CONNECTION: u32 = 0x00008000;
pub const CLIENT_MULTI_STATEMENTS: u32 = 0x00010000;
pub const CLIENT_MULTI_RESULTS: u32 = 0x00020000;
pub const CLIENT_PS_MULTI_RESULTS: u32 = 0x00040000;
pub const CLIENT_PLUGIN_AUTH: u32 = 0x00080000;
pub const CLIENT_CONNECT_ATTRS: u32 = 0x00100000;
pub const CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA: u32 = 0x00200000;
pub const CLIENT_CAN_HANDLE_EXPIRED_PASSWORDS: u32 = 0x00400000;
pub const CLIENT_SESSION_TRACK: u32 = 0x00800000;
pub const CLIENT_DEPRECATE_EOF: u32 = 0x01000000;

// Default server capabilities
pub const DEFAULT_CAPABILITY_FLAGS: u32 = CLIENT_LONG_PASSWORD
    | CLIENT_FOUND_ROWS
    | CLIENT_LONG_FLAG
    | CLIENT_CONNECT_WITH_DB
    | CLIENT_NO_SCHEMA
    | CLIENT_PROTOCOL_41
    | CLIENT_TRANSACTIONS
    | CLIENT_SECURE_CONNECTION
    | CLIENT_MULTI_STATEMENTS
    | CLIENT_MULTI_RESULTS
    | CLIENT_PLUGIN_AUTH
    | CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA;

// Character set
pub const UTF8MB4_GENERAL_CI: u8 = 45;
pub const UTF8MB4_0900_AI_CI: u8 = 255;

// Command types
pub const COM_SLEEP: u8 = 0x00;
pub const COM_QUIT: u8 = 0x01;
pub const COM_INIT_DB: u8 = 0x02;
pub const COM_QUERY: u8 = 0x03;
pub const COM_FIELD_LIST: u8 = 0x04;
pub const COM_CREATE_DB: u8 = 0x05;
pub const COM_DROP_DB: u8 = 0x06;
pub const COM_REFRESH: u8 = 0x07;
pub const COM_SHUTDOWN: u8 = 0x08;
pub const COM_STATISTICS: u8 = 0x09;
pub const COM_PROCESS_INFO: u8 = 0x0A;
pub const COM_CONNECT: u8 = 0x0B;
pub const COM_PROCESS_KILL: u8 = 0x0C;
pub const COM_DEBUG: u8 = 0x0D;
pub const COM_PING: u8 = 0x0E;
pub const COM_TIME: u8 = 0x0F;
pub const COM_DELAYED_INSERT: u8 = 0x10;
pub const COM_CHANGE_USER: u8 = 0x11;
pub const COM_STMT_PREPARE: u8 = 0x16;
pub const COM_STMT_EXECUTE: u8 = 0x17;
pub const COM_STMT_CLOSE: u8 = 0x19;
pub const COM_STMT_RESET: u8 = 0x1A;
pub const COM_SET_OPTION: u8 = 0x1B;

// Status flags
pub const SERVER_STATUS_IN_TRANS: u16 = 0x0001;
pub const SERVER_STATUS_AUTOCOMMIT: u16 = 0x0002;
pub const SERVER_MORE_RESULTS_EXISTS: u16 = 0x0008;
pub const SERVER_STATUS_NO_GOOD_INDEX_USED: u16 = 0x0010;
pub const SERVER_STATUS_NO_INDEX_USED: u16 = 0x0020;
pub const SERVER_STATUS_CURSOR_EXISTS: u16 = 0x0040;
pub const SERVER_STATUS_LAST_ROW_SENT: u16 = 0x0080;
pub const SERVER_STATUS_DB_DROPPED: u16 = 0x0100;
pub const SERVER_STATUS_NO_BACKSLASH_ESCAPES: u16 = 0x0200;
pub const SERVER_STATUS_METADATA_CHANGED: u16 = 0x0400;
pub const SERVER_QUERY_WAS_SLOW: u16 = 0x0800;
pub const SERVER_PS_OUT_PARAMS: u16 = 0x1000;
pub const SERVER_STATUS_IN_TRANS_READONLY: u16 = 0x2000;
pub const SERVER_SESSION_STATE_CHANGED: u16 = 0x4000;

// Column types
pub const MYSQL_TYPE_DECIMAL: u8 = 0x00;
pub const MYSQL_TYPE_TINY: u8 = 0x01;
pub const MYSQL_TYPE_SHORT: u8 = 0x02;
pub const MYSQL_TYPE_LONG: u8 = 0x03;
pub const MYSQL_TYPE_FLOAT: u8 = 0x04;
pub const MYSQL_TYPE_DOUBLE: u8 = 0x05;
pub const MYSQL_TYPE_NULL: u8 = 0x06;
pub const MYSQL_TYPE_TIMESTAMP: u8 = 0x07;
pub const MYSQL_TYPE_LONGLONG: u8 = 0x08;
pub const MYSQL_TYPE_INT24: u8 = 0x09;
pub const MYSQL_TYPE_DATE: u8 = 0x0A;
pub const MYSQL_TYPE_TIME: u8 = 0x0B;
pub const MYSQL_TYPE_DATETIME: u8 = 0x0C;
pub const MYSQL_TYPE_YEAR: u8 = 0x0D;
pub const MYSQL_TYPE_NEWDATE: u8 = 0x0E;
pub const MYSQL_TYPE_VARCHAR: u8 = 0x0F;
pub const MYSQL_TYPE_BIT: u8 = 0x10;
pub const MYSQL_TYPE_NEWDECIMAL: u8 = 0xF6;
pub const MYSQL_TYPE_ENUM: u8 = 0xF7;
pub const MYSQL_TYPE_SET: u8 = 0xF8;
pub const MYSQL_TYPE_TINY_BLOB: u8 = 0xF9;
pub const MYSQL_TYPE_MEDIUM_BLOB: u8 = 0xFA;
pub const MYSQL_TYPE_LONG_BLOB: u8 = 0xFB;
pub const MYSQL_TYPE_BLOB: u8 = 0xFC;
pub const MYSQL_TYPE_VAR_STRING: u8 = 0xFD;
pub const MYSQL_TYPE_STRING: u8 = 0xFE;
pub const MYSQL_TYPE_GEOMETRY: u8 = 0xFF;

// Column flags
pub const NOT_NULL_FLAG: u16 = 0x0001;
pub const PRI_KEY_FLAG: u16 = 0x0002;
pub const UNIQUE_KEY_FLAG: u16 = 0x0004;
pub const MULTIPLE_KEY_FLAG: u16 = 0x0008;
pub const BLOB_FLAG: u16 = 0x0010;
pub const UNSIGNED_FLAG: u16 = 0x0020;
pub const ZEROFILL_FLAG: u16 = 0x0040;
pub const BINARY_FLAG: u16 = 0x0080;
pub const ENUM_FLAG: u16 = 0x0100;
pub const AUTO_INCREMENT_FLAG: u16 = 0x0200;
pub const TIMESTAMP_FLAG: u16 = 0x0400;
pub const SET_FLAG: u16 = 0x0800;
pub const NO_DEFAULT_VALUE_FLAG: u16 = 0x1000;
pub const ON_UPDATE_NOW_FLAG: u16 = 0x2000;
pub const NUM_FLAG: u16 = 0x8000;
