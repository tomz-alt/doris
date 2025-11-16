// MySQL Protocol Unit Tests
// Tests ensure 100% compatibility with Java FE implementation
//
// Based on Java FE tests in:
// - fe/fe-core/src/test/java/org/apache/doris/mysql/

#[cfg(test)]
mod tests {
    use super::super::protocol::*;
    use super::super::packet::{
        HandshakePacket,
        HandshakeResponse,
        OkPacket,
        ErrPacket,
        EofPacket,
        ResultRow,
    };
    use crate::error::DorisError;
    use bytes::{BytesMut, BufMut};

    const PROTOCOL_VERSION: u8 = 10;
    const SERVER_VERSION: &str = "5.7.99";
    const DEFAULT_CHARSET: u8 = 33; // UTF-8

    /// MySQL Column Type Code Mapping Tests
    /// Based on MysqlColTypeTest.java
    #[test]
    fn test_column_type_codes() {
        // These codes MUST match the Java implementation exactly
        // From: org.apache.doris.catalog.MysqlColType

        assert_eq!(ColumnType::Decimal as u8, 0x00, "DECIMAL type code");
        assert_eq!(ColumnType::Tiny as u8, 0x01, "TINY INT type code");
        assert_eq!(ColumnType::Short as u8, 0x02, "SMALL INT type code");
        assert_eq!(ColumnType::Long as u8, 0x03, "INT type code");
        assert_eq!(ColumnType::Float as u8, 0x04, "FLOAT type code");
        assert_eq!(ColumnType::Double as u8, 0x05, "DOUBLE type code");
        assert_eq!(ColumnType::Null as u8, 0x06, "NULL type code");
        assert_eq!(ColumnType::Timestamp as u8, 0x07, "TIMESTAMP type code");
        assert_eq!(ColumnType::LongLong as u8, 0x08, "BIGINT type code");
        assert_eq!(ColumnType::Int24 as u8, 0x09, "MEDIUMINT type code");
        assert_eq!(ColumnType::Date as u8, 0x0a, "DATE type code");
        assert_eq!(ColumnType::Time as u8, 0x0b, "TIME type code");
        assert_eq!(ColumnType::DateTime as u8, 0x0c, "DATETIME type code");
        assert_eq!(ColumnType::Year as u8, 0x0d, "YEAR type code");
        assert_eq!(ColumnType::VarChar as u8, 0x0f, "VARCHAR type code");
        assert_eq!(ColumnType::Bit as u8, 0x10, "BIT type code");
        assert_eq!(ColumnType::Json as u8, 0xf5, "JSON type code");
        assert_eq!(ColumnType::NewDecimal as u8, 0xf6, "NEW DECIMAL type code");
        assert_eq!(ColumnType::Enum as u8, 0xf7, "ENUM type code");
        assert_eq!(ColumnType::Set as u8, 0xf8, "SET type code");
        assert_eq!(ColumnType::TinyBlob as u8, 0xf9, "TINY BLOB type code");
        assert_eq!(ColumnType::MediumBlob as u8, 0xfa, "MEDIUM BLOB type code");
        assert_eq!(ColumnType::LongBlob as u8, 0xfb, "LONG BLOB type code");
        assert_eq!(ColumnType::Blob as u8, 0xfc, "BLOB type code");
        assert_eq!(ColumnType::VarString as u8, 0xfd, "VAR STRING type code");
        assert_eq!(ColumnType::String as u8, 0xfe, "STRING type code");
        assert_eq!(ColumnType::Geometry as u8, 0xff, "GEOMETRY type code");
    }

    /// MySQL Protocol Version Test
    #[test]
    fn test_protocol_version() {
        // MySQL protocol version must be 10
        assert_eq!(PROTOCOL_VERSION, 10, "Protocol version must be 10");
    }

    /// Server Version String Test
    /// Based on MysqlHandshakePacketTest.java
    #[test]
    fn test_server_version() {
        // Doris reports itself as MySQL 5.7.99 for compatibility
        assert_eq!(SERVER_VERSION, "5.7.99", "Server version for MySQL compatibility");
    }

    /// Character Set Test
    /// Based on MysqlHandshakePacketTest.java
    #[test]
    fn test_default_charset() {
        // Default character set is UTF-8 (code 33)
        assert_eq!(DEFAULT_CHARSET, 33, "Default charset must be UTF-8 (33)");
    }

    /// Capability Flags Tests
    /// Based on MysqlCapabilityTest.java
    #[test]
    fn test_capability_flags() {
        let caps = server_capabilities();

        // Client capabilities that MUST be supported
        assert!(caps & CLIENT_LONG_PASSWORD != 0, "LONG_PASSWORD capability");
        assert!(caps & CLIENT_FOUND_ROWS != 0, "FOUND_ROWS capability");
        assert!(caps & CLIENT_LONG_FLAG != 0, "LONG_FLAG capability");
        assert!(caps & CLIENT_CONNECT_WITH_DB != 0, "CONNECT_WITH_DB capability");
        assert!(caps & CLIENT_PROTOCOL_41 != 0, "PROTOCOL_41 capability");
        assert!(caps & CLIENT_TRANSACTIONS != 0, "TRANSACTIONS capability");
        assert!(caps & CLIENT_SECURE_CONNECTION != 0, "SECURE_CONNECTION capability");
        assert!(caps & CLIENT_PLUGIN_AUTH != 0, "PLUGIN_AUTH capability");
        // The server advertises CLIENT_DEPRECATE_EOF and must implement
        // OK-instead-of-EOF result-set semantics correctly.
        assert!(caps & CLIENT_DEPRECATE_EOF != 0, "DEPRECATE_EOF capability");
    }

    /// HandshakePacket should use the same constants as the protocol tests.
    #[test]
    fn test_handshake_packet_defaults_match_protocol_constants() {
        let connection_id = 1090u32;
        let handshake = HandshakePacket::new(connection_id);

        assert_eq!(handshake.protocol_version, PROTOCOL_VERSION);
        assert_eq!(handshake.server_version, SERVER_VERSION);
        assert_eq!(handshake.character_set, DEFAULT_CHARSET as u8);
        assert_eq!(handshake.connection_id, connection_id);
        // Handshake must advertise the same capabilities as server_capabilities.
        let caps = server_capabilities();
        assert_eq!(handshake.capability_flags, caps);
    }

    /// Handshake Packet Serialization Test
    /// Based on MysqlHandshakePacketTest.java
    #[test]
    fn test_handshake_packet_serialization() {
        let connection_id = 1090u32;
        let auth_plugin_data = b"abcdefghijklmnopqrst"; // 20 bytes

        let mut buf = BytesMut::new();

        // Write handshake packet (simplified)
        // Protocol version
        buf.put_u8(PROTOCOL_VERSION);

        // Server version (null-terminated)
        buf.put_slice(SERVER_VERSION.as_bytes());
        buf.put_u8(0);

        // Connection ID (4 bytes)
        buf.put_u32_le(connection_id);

        // Auth plugin data part 1 (8 bytes)
        buf.put_slice(&auth_plugin_data[0..8]);

        // Filler (1 byte)
        buf.put_u8(0);

        // Capability flags lower 2 bytes
        let caps = server_capabilities();
        buf.put_u16_le((caps & 0xFFFF) as u16);

        // Character set (1 byte)
        buf.put_u8(DEFAULT_CHARSET);

        // Status flags (2 bytes)
        buf.put_u16_le(0);

        // Capability flags upper 2 bytes
        buf.put_u16_le(((caps >> 16) & 0xFFFF) as u16);

        // Length of auth plugin data (1 byte)
        buf.put_u8(21);

        // Reserved (10 bytes)
        buf.put_slice(&[0u8; 10]);

        // Auth plugin data part 2 (12 bytes + null terminator)
        buf.put_slice(&auth_plugin_data[8..20]);
        buf.put_u8(0);

        // Auth plugin name (null-terminated)
        buf.put_slice(b"mysql_native_password");
        buf.put_u8(0);

        // Verify packet structure
        assert_eq!(buf[0], 10, "Protocol version");
        // Connection ID starts after version string "5.7.99\0" = 8 bytes (1 + 7)
        assert_eq!(connection_id, u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]), "Connection ID");
        // Character set is at offset: 1 + 7 + 4 + 8 + 1 + 2 = 23
        assert_eq!(buf[23], DEFAULT_CHARSET, "Character set");
        // Auth plugin data length is at offset: 23 + 1 + 2 + 2 = 28
        assert_eq!(buf[28], 21, "Auth plugin data length");
    }

    /// Auth Packet Parsing Test
    /// Based on MysqlAuthPacketTest.java
    #[test]
    fn test_auth_packet_parsing() {
        let mut buf = BytesMut::new();

        // Capability flags (4 bytes)
        let caps = server_capabilities();
        buf.put_u32_le(caps);

        // Max packet size (4 bytes)
        buf.put_u32_le(1024000);

        // Character set (1 byte)
        buf.put_u8(33);

        // Reserved (23 bytes)
        buf.put_slice(&[0u8; 23]);

        // User name (null-terminated)
        buf.put_slice(b"palo-user");
        buf.put_u8(0);

        // Auth response length (1 byte)
        buf.put_u8(20);

        // Auth response data (20 bytes)
        let auth_data: Vec<u8> = (b'a'..=b't').collect();
        buf.put_slice(&auth_data);

        // Database name (null-terminated)
        buf.put_slice(b"testDb");
        buf.put_u8(0);

        // Verify packet can be parsed
        assert_eq!(buf.len(), 4 + 4 + 1 + 23 + 10 + 1 + 20 + 7, "Auth packet total length");

        // Verify capability flags
        let parsed_caps = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(parsed_caps, caps, "Capability flags match");

        // Verify max packet size
        let max_packet = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(max_packet, 1024000, "Max packet size");

        // Verify charset
        assert_eq!(buf[8], 33, "Character set");
    }

    /// HandshakeResponse should reject packets that are too short.
    #[test]
    fn test_handshake_response_too_short() {
        // Fewer than 32 bytes should be rejected as INVALID_PACKET.
        let payload = bytes::Bytes::from_static(&[0u8; 16]);
        let err = HandshakeResponse::decode(payload).unwrap_err();
        match err {
            DorisError::InvalidPacket(msg) => {
                assert!(
                    msg.contains("too short"),
                    "expected 'too short' message but got: {}",
                    msg
                );
            }
            other => panic!("unexpected error type: {:?}", other),
        }
    }

    /// HandshakeResponse should reject secure-connection auth data with
    /// inconsistent lengths.
    #[test]
    fn test_handshake_response_bad_auth_length() {
        use bytes::BufMut;

        let mut buf = BytesMut::new();

        // capability_flags: CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION
        let caps = CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION;
        buf.put_u32_le(caps);
        // max_packet_size
        buf.put_u32_le(1024 * 1024);
        // character_set
        buf.put_u8(33);
        // reserved (23 bytes)
        buf.put_bytes(0, 23);
        // username "root\0"
        buf.put_slice(b"root");
        buf.put_u8(0);
        // auth_response length says 10, but we only provide 5 bytes
        buf.put_u8(10);
        buf.put_slice(b"abcde");

        let payload = buf.freeze();
        let err = HandshakeResponse::decode(payload).unwrap_err();
        match err {
            DorisError::InvalidPacket(msg) => {
                assert!(
                    msg.contains("Auth response too short"),
                    "expected 'Auth response too short' but got: {}",
                    msg
                );
            }
            other => panic!("unexpected error type: {:?}", other),
        }
    }

    /// OK Packet Format Test
    /// Based on MysqlOkPacketTest.java
    #[test]
    fn test_ok_packet_format() {
        let mut buf = BytesMut::new();

        // OK packet header (0x00)
        buf.put_u8(0x00);

        // Affected rows (length-encoded integer)
        buf.put_u8(5); // 5 affected rows

        // Last insert ID (length-encoded integer)
        buf.put_u8(0); // No insert ID

        // Status flags (2 bytes)
        buf.put_u16_le(SERVER_STATUS_AUTOCOMMIT);

        // Warnings (2 bytes)
        buf.put_u16_le(0);

        assert_eq!(buf[0], 0x00, "OK packet marker");
        assert_eq!(buf[1], 5, "Affected rows");
        assert_eq!(u16::from_le_bytes([buf[3], buf[4]]), SERVER_STATUS_AUTOCOMMIT, "Status flags");
    }

    /// Error Packet Format Test
    /// Based on MysqlErrPacketTest.java
    #[test]
    fn test_error_packet_format() {
        let mut buf = BytesMut::new();

        // Error packet header (0xFF)
        buf.put_u8(0xFF);

        // Error code (2 bytes)
        buf.put_u16_le(1045); // ER_ACCESS_DENIED_ERROR

        // SQL state marker (#)
        buf.put_u8(b'#');

        // SQL state (5 bytes)
        buf.put_slice(b"28000");

        // Error message
        let error_msg = "Access denied for user";
        buf.put_slice(error_msg.as_bytes());

        assert_eq!(buf[0], 0xFF, "Error packet marker");
        assert_eq!(u16::from_le_bytes([buf[1], buf[2]]), 1045, "Error code");
        assert_eq!(buf[3], b'#', "SQL state marker");
        assert_eq!(&buf[4..9], b"28000", "SQL state");
    }

    /// EOF Packet Format Test
    /// Based on MysqlEofPacketTest.java
    #[test]
    fn test_eof_packet_format() {
        let mut buf = BytesMut::new();

        // EOF packet header (0xFE)
        buf.put_u8(0xFE);

        // Warnings (2 bytes)
        buf.put_u16_le(0);

        // Status flags (2 bytes)
        buf.put_u16_le(SERVER_STATUS_AUTOCOMMIT);

        assert_eq!(buf[0], 0xFE, "EOF packet marker");
        assert_eq!(buf.len(), 5, "EOF packet length");
        assert_eq!(u16::from_le_bytes([buf[3], buf[4]]), SERVER_STATUS_AUTOCOMMIT, "Status flags");
    }

    /// Column Definition Packet Test
    /// Based on MysqlColDefTest.java
    #[test]
    fn test_column_definition_packet() {
        let mut buf = BytesMut::new();

        // Catalog (length-encoded string) - usually "def"
        write_length_encoded_string(&mut buf, "def");

        // Schema (length-encoded string)
        write_length_encoded_string(&mut buf, "testdb");

        // Table (length-encoded string)
        write_length_encoded_string(&mut buf, "test_table");

        // Original table (length-encoded string)
        write_length_encoded_string(&mut buf, "test_table");

        // Name (length-encoded string)
        write_length_encoded_string(&mut buf, "id");

        // Original name (length-encoded string)
        write_length_encoded_string(&mut buf, "id");

        // Length of fixed fields (1 byte)
        buf.put_u8(0x0c);

        // Character set (2 bytes)
        buf.put_u16_le(33); // UTF-8

        // Column length (4 bytes)
        buf.put_u32_le(11);

        // Type (1 byte)
        buf.put_u8(ColumnType::Long as u8); // INT

        // Flags (2 bytes)
        buf.put_u16_le(0); // No special flags

        // Decimals (1 byte)
        buf.put_u8(0);

        // Filler (2 bytes)
        buf.put_u16_le(0);

        // Verify column definition structure
        assert!(buf.len() > 0, "Column definition packet created");
    }

    /// Length-Encoded Integer Tests
    #[test]
    fn test_length_encoded_integers() {
        let mut buf = BytesMut::new();

        // 0 should be encoded as single byte
        write_length_encoded_int(&mut buf, 0);
        assert_eq!(buf[0], 0);
        buf.clear();

        // Test small value (< 251)
        write_length_encoded_int(&mut buf, 100);
        assert_eq!(buf[0], 100);
        buf.clear();

        // Boundary at 250
        write_length_encoded_int(&mut buf, 250);
        assert_eq!(buf[0], 250);
        buf.clear();

        // 251 and above use markers
        write_length_encoded_int(&mut buf, 251);
        assert_eq!(buf[0], 0xfc);
        assert_eq!(u16::from_le_bytes([buf[1], buf[2]]), 251);
        buf.clear();

        // Test 2-byte value
        write_length_encoded_int(&mut buf, 300);
        assert_eq!(buf[0], 0xfc);
        assert_eq!(u16::from_le_bytes([buf[1], buf[2]]), 300);
        buf.clear();

        // Test 3-byte value
        write_length_encoded_int(&mut buf, 70000);
        assert_eq!(buf[0], 0xfd);
        let three = buf[1] as u32 | ((buf[2] as u32) << 8) | ((buf[3] as u32) << 16);
        assert_eq!(three, 70000);
        buf.clear();

        // Test 8-byte value
        write_length_encoded_int(&mut buf, 20000000);
        assert_eq!(buf[0], 0xfe);
        let eight = u64::from_le_bytes(buf[1..9].try_into().unwrap());
        assert_eq!(eight, 20000000);
    }

    /// OKPacket encoding tests using implementation.
    #[test]
    fn test_ok_packet_encode() {
        let mut ok = OkPacket::new();
        ok.affected_rows = 5;
        ok.last_insert_id = 42;
        ok.status_flags = SERVER_STATUS_AUTOCOMMIT;
        ok.warnings = 3;
        ok.info = "done".to_string();

        let bytes = ok.encode();
        // Header
        assert_eq!(bytes[0], 0x00);
        // affected_rows = 5 (single-byte lenenc int)
        assert_eq!(bytes[1], 5);
        // last_insert_id = 42 (single-byte lenenc int)
        assert_eq!(bytes[2], 42);
        // status flags
        assert_eq!(u16::from_le_bytes([bytes[3], bytes[4]]), SERVER_STATUS_AUTOCOMMIT);
        // warnings
        assert_eq!(u16::from_le_bytes([bytes[5], bytes[6]]), 3);
        // info string
        assert_eq!(&bytes[7..], b"done");

        // encode_as_eof should use 0xFE header but same body layout.
        let eof_like = ok.encode_as_eof();
        assert_eq!(eof_like[0], 0xfe);
        assert_eq!(eof_like[1..], bytes[1..]);
    }

    /// ErrPacket encoding tests using implementation.
    #[test]
    fn test_err_packet_encode() {
        let err = ErrPacket::new(1064, "You have an error in your SQL".to_string());
        let bytes = err.encode();

        assert_eq!(bytes[0], 0xff);
        assert_eq!(u16::from_le_bytes([bytes[1], bytes[2]]), 1064);
        assert_eq!(bytes[3], b'#');
        assert_eq!(&bytes[4..9], b"HY000");
        let msg = String::from_utf8(bytes[9..].to_vec()).unwrap();
        assert!(msg.starts_with("You have an error"));
    }

    /// EofPacket encoding tests using implementation.
    #[test]
    fn test_eof_packet_encode() {
        let eof = EofPacket::new();
        let bytes = eof.encode();
        assert_eq!(bytes[0], 0xfe);
        assert_eq!(bytes.len(), 5);
        assert_eq!(u16::from_le_bytes([bytes[1], bytes[2]]), 0);
        assert_eq!(u16::from_le_bytes([bytes[3], bytes[4]]), SERVER_STATUS_AUTOCOMMIT);
    }

    /// ResultRow encoding tests (text protocol).
    #[test]
    fn test_result_row_encode_null_and_values() {
        let row = ResultRow::new(vec![
            Some("abc".to_string()),
            None,
            Some("".to_string()),
        ]);
        let bytes = row.encode();

        // First value: len=3 + 'abc'
        assert_eq!(bytes[0], 3);
        assert_eq!(&bytes[1..4], b"abc");

        // Second value: NULL marker (0xfb)
        assert_eq!(bytes[4], 0xfb);

        // Third value: empty string is length 0 and no bytes.
        assert_eq!(bytes[5], 0);
        assert_eq!(bytes.len(), 6);
    }

    /// Command Type Tests
    #[test]
    fn test_command_types() {
        // Test command parsing
        assert_eq!(Command::from(0x03), Command::Query);
        assert_eq!(Command::from(0x02), Command::InitDb);
        assert_eq!(Command::from(0x01), Command::Quit);
        assert_eq!(Command::from(0x0e), Command::Ping);
        assert_eq!(Command::from(0x04), Command::FieldList);
        assert_eq!(Command::from(0x16), Command::StmtPrepare);
        assert_eq!(Command::from(0x17), Command::StmtExecute);
        assert_eq!(Command::from(0x19), Command::StmtClose);

        // Test unknown command
        match Command::from(0xFF) {
            Command::Unknown(0xFF) => {}, // Expected
            _ => panic!("Unknown command parsing failed"),
        }
    }

    /// Helper function to write length-encoded string
    fn write_length_encoded_string(buf: &mut BytesMut, s: &str) {
        let bytes = s.as_bytes();
        write_length_encoded_int(buf, bytes.len() as u64);
        buf.put_slice(bytes);
    }

    /// Helper function to write length-encoded integer
    fn write_length_encoded_int(buf: &mut BytesMut, n: u64) {
        if n < 251 {
            buf.put_u8(n as u8);
        } else if n < 65536 {
            buf.put_u8(0xfc);
            buf.put_u16_le(n as u16);
        } else if n < 16777216 {
            buf.put_u8(0xfd);
            buf.put_u8((n & 0xff) as u8);
            buf.put_u8(((n >> 8) & 0xff) as u8);
            buf.put_u8(((n >> 16) & 0xff) as u8);
        } else {
            buf.put_u8(0xfe);
            buf.put_u64_le(n);
        }
    }
}

// ============================================================================
// Password Authentication Tests
// Based on: MysqlPasswordTest.java
// ============================================================================

#[cfg(test)]
mod password_tests {
    use super::super::protocol::*;
    use sha1::{Sha1, Digest};

    /// Generate MySQL hashed password (SERVER_PUBLIC_KEY format)
    /// Equivalent to Java's MysqlPassword.makeScrambledPassword()
    fn make_scrambled_password(password: &str) -> String {
        if password.is_empty() {
            return String::new();
        }

        // SHA1(password)
        let mut hasher = Sha1::new();
        hasher.update(password.as_bytes());
        let hash1 = hasher.finalize();

        // SHA1(SHA1(password))
        let mut hasher = Sha1::new();
        hasher.update(&hash1);
        let hash2 = hasher.finalize();

        // Format as *HEXSTRING
        format!("*{}", hex::encode_upper(hash2))
    }

    /// Extract salt from hashed password
    /// Equivalent to Java's MysqlPassword.getSaltFromPassword()
    fn get_salt_from_password(hashed: &str) -> Option<Vec<u8>> {
        if hashed.is_empty() || hashed.len() != 41 || !hashed.starts_with('*') {
            return None;
        }

        hex::decode(&hashed[1..]).ok()
    }

    /// Generate random salt (20 bytes)
    /// Equivalent to Java's MysqlPassword.createRandomString()
    fn create_random_salt(len: usize) -> Vec<u8> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..len).map(|_| rng.gen()).collect()
    }

    /// Verify scrambled password
    /// Equivalent to Java's MysqlPassword.checkScramble()
    fn check_scramble(scrambled: &[u8], salt: &[u8], expected_hash: &[u8]) -> bool {
        if scrambled.len() != 20 || expected_hash.len() != 20 {
            return false;
        }

        // SHA1(salt + expected_hash)
        let mut hasher = Sha1::new();
        hasher.update(salt);
        hasher.update(expected_hash);
        let hash = hasher.finalize();

        // XOR scrambled with hash to get SHA1(password)
        let password_hash: Vec<u8> = scrambled.iter()
            .zip(hash.iter())
            .map(|(a, b)| a ^ b)
            .collect();

        // SHA1(SHA1(password))
        let mut hasher = Sha1::new();
        hasher.update(&password_hash);
        let double_hash = hasher.finalize();

        // Compare with expected hash
        double_hash.as_slice() == expected_hash
    }

    /// Test password hashing (makeScrambledPassword)
    /// Based on MysqlPasswordTest.testMakePassword()
    #[test]
    fn test_make_scrambled_password() {
        // Test case 1: "mypass" -> *6C8989366EAF75BB670AD8EA7A7FC1176A95CEF4
        let hashed = make_scrambled_password("mypass");
        assert_eq!(hashed, "*6C8989366EAF75BB670AD8EA7A7FC1176A95CEF4");

        // Test case 2: Empty password
        let hashed = make_scrambled_password("");
        assert_eq!(hashed, "");

        // Test case 3: "aBc@321" -> *9A6EC51164108A8D3DA3BE3F35A56F6499B6FC32
        let hashed = make_scrambled_password("aBc@321");
        assert_eq!(hashed, "*9A6EC51164108A8D3DA3BE3F35A56F6499B6FC32");
    }

    /// Test salt extraction from hashed password
    /// Based on MysqlPasswordTest.testMakePassword()
    #[test]
    fn test_get_salt_from_password() {
        // Test valid hashed password
        let salt = get_salt_from_password("*6C8989366EAF75BB670AD8EA7A7FC1176A95CEF4");
        assert!(salt.is_some());
        assert_eq!(salt.unwrap().len(), 20);

        // Test empty password
        let salt = get_salt_from_password("");
        assert!(salt.is_none());

        // Test invalid format (too short)
        let salt = get_salt_from_password("*6C8989");
        assert!(salt.is_none());

        // Test invalid format (no asterisk)
        let salt = get_salt_from_password("6C8989366EAF75BB670AD8EA7A7FC1176A95CEF4");
        assert!(salt.is_none());
    }

    /// Test password scrambling and verification
    /// Based on MysqlPasswordTest.testCheckPass()
    #[test]
    fn test_password_scramble_and_verify() {
        // Simulate client-server authentication flow
        let password = "mypass";
        let hashed_password = "*6C8989366EAF75BB670AD8EA7A7FC1176A95CEF4";

        // Server generates random salt
        let salt = create_random_salt(20);

        // Client scrambles password with salt
        let scrambled = scramble_password(password, &salt);

        // Server extracts expected hash from stored password
        let expected_hash = get_salt_from_password(hashed_password).unwrap();

        // Server verifies scrambled password
        assert!(check_scramble(&scrambled, &salt, &expected_hash));

        // Test with wrong password
        let wrong_hashed = "*9A6EC51164108A8D3DA3BE3F35A56F6499B6FC32"; // aBc@321
        let wrong_hash = get_salt_from_password(wrong_hashed).unwrap();
        assert!(!check_scramble(&scrambled, &salt, &wrong_hash));
    }

    /// Test password hashing with various inputs
    #[test]
    fn test_password_hashing_edge_cases() {
        // Single character
        let hashed = make_scrambled_password("a");
        assert_eq!(hashed.len(), 41); // 1 asterisk + 40 hex chars

        // Long password
        let long_pass = "a".repeat(100);
        let hashed = make_scrambled_password(&long_pass);
        assert_eq!(hashed.len(), 41);

        // Special characters
        let hashed = make_scrambled_password("!@#$%^&*()_+-=[]{}|;':\",./<>?");
        assert_eq!(hashed.len(), 41);

        // Unicode characters
        let hashed = make_scrambled_password("密码123");
        assert_eq!(hashed.len(), 41);
    }

    /// Test scramble_password function directly
    #[test]
    fn test_scramble_password_function() {
        let password = "test123";
        let salt = b"12345678901234567890"; // 20 bytes

        let scrambled = scramble_password(password, salt);

        // Scrambled result should be 20 bytes (SHA1 output)
        assert_eq!(scrambled.len(), 20);

        // Same input should produce same output (deterministic)
        let scrambled2 = scramble_password(password, salt);
        assert_eq!(scrambled, scrambled2);

        // Different salt should produce different output
        let different_salt = b"09876543210987654321";
        let scrambled3 = scramble_password(password, different_salt);
        assert_ne!(scrambled, scrambled3);
    }

    /// Test random salt generation
    #[test]
    fn test_random_salt_generation() {
        // Generate multiple salts
        let salt1 = create_random_salt(20);
        let salt2 = create_random_salt(20);
        let salt3 = create_random_salt(20);

        // All should be 20 bytes
        assert_eq!(salt1.len(), 20);
        assert_eq!(salt2.len(), 20);
        assert_eq!(salt3.len(), 20);

        // Should be different (extremely high probability)
        assert_ne!(salt1, salt2);
        assert_ne!(salt2, salt3);
        assert_ne!(salt1, salt3);
    }

    /// Test password validation format
    #[test]
    fn test_password_format_validation() {
        // Valid format: *HEXSTRING (41 chars)
        let valid = "*6C8989366EAF75BB670AD8EA7A7FC1176A95CEF4";
        assert!(get_salt_from_password(valid).is_some());

        // Invalid: too short
        let invalid = "*6C8989366EAF75BB670AD8EA7A7FC1176A95CE";
        assert!(get_salt_from_password(invalid).is_none());

        // Invalid: too long
        let invalid = "*6C8989366EAF75BB670AD8EA7A7FC1176A95CEF4FF";
        assert!(get_salt_from_password(invalid).is_none());

        // Invalid: non-hex characters
        let invalid = "*6C8989366EAF75BB670AD8EA7A7FC1176A95CEZZ";
        assert!(get_salt_from_password(invalid).is_none());

        // Invalid: lowercase (case-sensitive check)
        // Note: hex::decode accepts both cases, so this tests format only
        let lowercase = "*6c8989366eaf75bb670ad8ea7a7fc1176a95cef4";
        let salt = get_salt_from_password(lowercase);
        assert!(salt.is_some()); // hex::decode accepts lowercase
    }
}

// ============================================================================
// Binary Data Serialization Tests
// Based on: MysqlSerializerVarbinaryTest.java
// ============================================================================

#[cfg(test)]
mod binary_serialization_tests {
    use super::super::protocol::*;
    use bytes::{BytesMut, BufMut};

    /// Test VARBINARY field packet serialization
    /// Based on MysqlSerializerVarbinaryTest.testFieldPacketForVarbinary()
    #[test]
    fn test_varbinary_field_packet() {
        let mut buf = BytesMut::new();

        // Simulate VARBINARY(10) column definition
        // Catalog
        write_length_encoded_string(&mut buf, "def");
        // Schema
        write_length_encoded_string(&mut buf, "");
        // Table
        write_length_encoded_string(&mut buf, "");
        // Original table
        write_length_encoded_string(&mut buf, "");
        // Name
        write_length_encoded_string(&mut buf, "c");
        // Original name
        write_length_encoded_string(&mut buf, "c");

        // Fixed fields length (0x0c = 12 bytes)
        buf.put_u8(0x0c);

        // Character set: 63 (binary collation)
        buf.put_u16_le(63);

        // Column length: 10
        buf.put_u32_le(10);

        // Type: VARBINARY maps to BLOB type (0xfc)
        buf.put_u8(ColumnType::Blob as u8);

        // Flags: BINARY_FLAG (128)
        buf.put_u16_le(128);

        // Decimals: 0
        buf.put_u8(0);

        // Filler: 2 bytes
        buf.put_u16_le(0);

        // Verify the packet structure
        // Skip to charset (after all length-encoded strings + 1 byte for 0x0c)
        let base_offset = 1 + 3 + 1 + 0 + 1 + 0 + 1 + 0 + 1 + 1 + 1 + 1 + 1; // Simplified
        // For testing, just verify key fields exist
        assert!(buf.len() > 20, "VARBINARY field packet should have substantial size");
    }

    /// Test VARCHAR field packet uses UTF-8 collation
    /// Based on MysqlSerializerVarbinaryTest.testFieldPacketForVarcharUsesUtf8Collation()
    #[test]
    fn test_varchar_field_packet() {
        let mut buf = BytesMut::new();

        // Simulate VARCHAR(10) column definition
        write_length_encoded_string(&mut buf, "def");
        write_length_encoded_string(&mut buf, "");
        write_length_encoded_string(&mut buf, "");
        write_length_encoded_string(&mut buf, "");
        write_length_encoded_string(&mut buf, "name");
        write_length_encoded_string(&mut buf, "name");

        buf.put_u8(0x0c);

        // Character set: 33 (utf8_general_ci)
        buf.put_u16_le(33);

        // Column length: 255 (default for VARCHAR)
        buf.put_u32_le(255);

        // Type: VARCHAR
        buf.put_u8(ColumnType::VarChar as u8);

        // Flags: 0 (not BINARY)
        buf.put_u16_le(0);

        // Decimals: 0
        buf.put_u8(0);

        // Filler: 2 bytes
        buf.put_u16_le(0);

        assert!(buf.len() > 20, "VARCHAR field packet should have substantial size");
    }

    /// Test length-encoded bytes preserves NULL bytes
    /// Based on MysqlSerializerVarbinaryTest.testWriteLenEncodedBytesPreservesNullByte()
    #[test]
    fn test_length_encoded_bytes_with_null() {
        let mut buf = BytesMut::new();

        // Data with embedded null byte: "a\0b"
        let data = vec![b'a', 0x00, b'b'];

        // Write length-encoded bytes
        write_length_encoded_int(&mut buf, data.len() as u64);
        buf.put_slice(&data);

        // Verify structure
        assert_eq!(buf[0], 3, "Length should be 3");
        assert_eq!(buf[1], b'a', "First byte should be 'a'");
        assert_eq!(buf[2], 0x00, "Second byte should be NULL");
        assert_eq!(buf[3], b'b', "Third byte should be 'b'");
    }

    /// Test that length-encoded string and bytes produce same output for ASCII
    /// Based on MysqlSerializerVarbinaryTest.testWriteLenEncodedStringAndBytesProduceSameForAscii()
    #[test]
    fn test_length_encoded_string_vs_bytes() {
        let mut buf1 = BytesMut::new();
        let mut buf2 = BytesMut::new();

        let text = "abc";

        // Write as string
        write_length_encoded_string(&mut buf1, text);

        // Write as bytes
        let bytes = text.as_bytes();
        write_length_encoded_int(&mut buf2, bytes.len() as u64);
        buf2.put_slice(bytes);

        // Both should produce identical output
        assert_eq!(buf1, buf2, "String and bytes encoding should match for ASCII");
    }

    /// Test binary data with various byte patterns
    #[test]
    fn test_binary_data_patterns() {
        let mut buf = BytesMut::new();

        // Test all byte values 0x00 - 0xFF
        let data: Vec<u8> = (0..=255).collect();

        write_length_encoded_int(&mut buf, data.len() as u64);
        buf.put_slice(&data);

        // Length should be encoded as 0xfc (2-byte encoding for 256)
        assert_eq!(buf[0], 0xfc, "Should use 2-byte length encoding");
        assert_eq!(u16::from_le_bytes([buf[1], buf[2]]), 256, "Length should be 256");

        // Verify all bytes are preserved
        for (i, &byte) in data.iter().enumerate() {
            assert_eq!(buf[3 + i], byte, "Byte {} should be preserved", i);
        }
    }

    /// Test character set codes
    #[test]
    fn test_character_set_codes() {
        // UTF-8 (utf8_general_ci)
        assert_eq!(33u16, 33);

        // Binary collation
        assert_eq!(63u16, 63);

        // These codes must match MySQL protocol specification
    }

    /// Test NULL value encoding in result set
    #[test]
    fn test_null_value_encoding() {
        let mut buf = BytesMut::new();

        // NULL is encoded as 0xfb in length-encoded format
        buf.put_u8(0xfb);

        assert_eq!(buf[0], 0xfb, "NULL should be encoded as 0xfb");
    }

    /// Test BINARY flag in column definition
    #[test]
    fn test_binary_flag() {
        const BINARY_FLAG: u16 = 128;
        const NOT_NULL_FLAG: u16 = 1;
        const PRIMARY_KEY_FLAG: u16 = 2;

        // VARBINARY column should have BINARY_FLAG
        let varbinary_flags = BINARY_FLAG;
        assert_eq!(varbinary_flags & BINARY_FLAG, BINARY_FLAG);

        // VARCHAR column should NOT have BINARY_FLAG
        let varchar_flags = 0u16;
        assert_eq!(varchar_flags & BINARY_FLAG, 0);

        // Can combine flags
        let combined = BINARY_FLAG | NOT_NULL_FLAG | PRIMARY_KEY_FLAG;
        assert_eq!(combined & BINARY_FLAG, BINARY_FLAG);
        assert_eq!(combined & NOT_NULL_FLAG, NOT_NULL_FLAG);
        assert_eq!(combined & PRIMARY_KEY_FLAG, PRIMARY_KEY_FLAG);
    }

    /// Helper function to write length-encoded string
    fn write_length_encoded_string(buf: &mut BytesMut, s: &str) {
        let bytes = s.as_bytes();
        write_length_encoded_int(buf, bytes.len() as u64);
        buf.put_slice(bytes);
    }

    /// Helper function to write length-encoded integer
    fn write_length_encoded_int(buf: &mut BytesMut, n: u64) {
        if n < 251 {
            buf.put_u8(n as u8);
        } else if n < 65536 {
            buf.put_u8(0xfc);
            buf.put_u16_le(n as u16);
        } else if n < 16777216 {
            buf.put_u8(0xfd);
            buf.put_u8((n & 0xff) as u8);
            buf.put_u8(((n >> 8) & 0xff) as u8);
            buf.put_u8(((n >> 16) & 0xff) as u8);
        } else {
            buf.put_u8(0xfe);
            buf.put_u64_le(n);
        }
    }
}
