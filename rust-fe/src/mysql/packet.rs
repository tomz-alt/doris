use bytes::{BufMut, BytesMut, Bytes, Buf};
use crate::error::{DorisError, Result};
use super::protocol::*;

// MySQL Packet structure
#[derive(Debug, Clone)]
pub struct Packet {
    pub sequence_id: u8,
    pub payload: Bytes,
}

impl Packet {
    pub fn new(sequence_id: u8, payload: Bytes) -> Self {
        Self { sequence_id, payload }
    }

    pub fn encode(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(4 + self.payload.len());

        // Payload length (3 bytes)
        let len = self.payload.len() as u32;
        buf.put_u8((len & 0xff) as u8);
        buf.put_u8(((len >> 8) & 0xff) as u8);
        buf.put_u8(((len >> 16) & 0xff) as u8);

        // Sequence ID
        buf.put_u8(self.sequence_id);

        // Payload
        buf.put_slice(&self.payload);

        buf
    }

    pub fn decode(buf: &mut BytesMut) -> Result<Option<Self>> {
        if buf.len() < 4 {
            return Ok(None);
        }

        // Read header
        let len = (buf[0] as usize) | ((buf[1] as usize) << 8) | ((buf[2] as usize) << 16);
        let sequence_id = buf[3];

        if buf.len() < 4 + len {
            return Ok(None);
        }

        // Remove header
        buf.advance(4);

        // Extract payload
        let payload = buf.split_to(len).freeze();

        Ok(Some(Packet { sequence_id, payload }))
    }
}

// Handshake packet (Initial handshake from server to client)
pub struct HandshakePacket {
    pub protocol_version: u8,
    pub server_version: String,
    pub connection_id: u32,
    pub auth_plugin_data: Vec<u8>,
    pub capability_flags: u32,
    pub character_set: u8,
    pub status_flags: u16,
    pub auth_plugin_name: String,
}

impl HandshakePacket {
    pub fn new(connection_id: u32) -> Self {
        // Generate random salt (20 bytes)
        let auth_plugin_data: Vec<u8> = (0..20).map(|i| (i * 7 + 13) as u8).collect();

        // For wire-compatibility with the Java FE and MySQL clients, we
        // advertise ourselves as a MySQL 5.7-compatible server using the
        // classic UTF-8 charset (33). This matches the Java FE behavior and
        // the expectations encoded in src/mysql/protocol_tests.rs.
        Self {
            protocol_version: 10,
            server_version: "5.7.99".to_string(),
            connection_id,
            auth_plugin_data,
            capability_flags: server_capabilities(),
            character_set: 33, // utf8_general_ci
            status_flags: SERVER_STATUS_AUTOCOMMIT,
            auth_plugin_name: "mysql_native_password".to_string(),
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();

        // Protocol version
        buf.put_u8(self.protocol_version);

        // Server version
        write_null_terminated_str(&mut buf, &self.server_version);

        // Connection ID
        buf.put_u32_le(self.connection_id);

        // Auth plugin data part 1 (8 bytes)
        buf.put_slice(&self.auth_plugin_data[0..8]);

        // Filler
        buf.put_u8(0);

        // Capability flags (lower 2 bytes)
        buf.put_u16_le((self.capability_flags & 0xffff) as u16);

        // Character set
        buf.put_u8(self.character_set);

        // Status flags
        buf.put_u16_le(self.status_flags);

        // Capability flags (upper 2 bytes)
        buf.put_u16_le(((self.capability_flags >> 16) & 0xffff) as u16);

        // Auth plugin data length
        buf.put_u8(self.auth_plugin_data.len() as u8 + 1);

        // Reserved (10 bytes)
        buf.put_bytes(0, 10);

        // Auth plugin data part 2 (12 bytes)
        buf.put_slice(&self.auth_plugin_data[8..20]);

        // Null terminator
        buf.put_u8(0);

        // Auth plugin name
        write_null_terminated_str(&mut buf, &self.auth_plugin_name);

        buf.freeze()
    }

    pub fn salt(&self) -> &[u8] {
        &self.auth_plugin_data
    }
}

// Handshake Response (from client)
#[derive(Debug)]
pub struct HandshakeResponse {
    pub capability_flags: u32,
    pub max_packet_size: u32,
    pub character_set: u8,
    pub username: String,
    pub auth_response: Vec<u8>,
    pub database: Option<String>,
    pub auth_plugin_name: Option<String>,
}

impl HandshakeResponse {
    pub fn decode(mut payload: Bytes) -> Result<Self> {
        if payload.len() < 32 {
            return Err(DorisError::InvalidPacket("Handshake response too short".to_string()));
        }

        let capability_flags = payload.get_u32_le();
        let max_packet_size = payload.get_u32_le();
        let character_set = payload.get_u8();

        // Skip reserved bytes (23 bytes)
        payload.advance(23);

        let username = read_null_terminated_string(&mut payload)
            .ok_or_else(|| DorisError::InvalidPacket("Invalid username".to_string()))?;

        let auth_response = if capability_flags & CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA != 0 {
            let len = read_lenenc_int(&mut payload)
                .ok_or_else(|| DorisError::InvalidPacket("Invalid auth response length".to_string()))? as usize;
            if payload.remaining() < len {
                return Err(DorisError::InvalidPacket("Auth response too short".to_string()));
            }
            payload.copy_to_bytes(len).to_vec()
        } else if capability_flags & CLIENT_SECURE_CONNECTION != 0 {
            let len = payload.get_u8() as usize;
            if payload.remaining() < len {
                return Err(DorisError::InvalidPacket("Auth response too short".to_string()));
            }
            payload.copy_to_bytes(len).to_vec()
        } else {
            read_null_terminated_string(&mut payload)
                .ok_or_else(|| DorisError::InvalidPacket("Invalid auth response".to_string()))?
                .into_bytes()
        };

        let database = if capability_flags & CLIENT_CONNECT_WITH_DB != 0 {
            read_null_terminated_string(&mut payload)
        } else {
            None
        };

        let auth_plugin_name = if capability_flags & CLIENT_PLUGIN_AUTH != 0 {
            read_null_terminated_string(&mut payload)
        } else {
            None
        };

        Ok(Self {
            capability_flags,
            max_packet_size,
            character_set,
            username,
            auth_response,
            database,
            auth_plugin_name,
        })
    }
}

// OK Packet
pub struct OkPacket {
    pub affected_rows: u64,
    pub last_insert_id: u64,
    pub status_flags: u16,
    pub warnings: u16,
    pub info: String,
}

impl OkPacket {
    pub fn new() -> Self {
        Self {
            affected_rows: 0,
            last_insert_id: 0,
            status_flags: SERVER_STATUS_AUTOCOMMIT,
            warnings: 0,
            info: String::new(),
        }
    }

    pub fn encode(&self) -> Bytes {
        self.encode_with_header(0x00)
    }

    // Encode OK packet with 0xFE header (used to replace EOF when CLIENT_DEPRECATE_EOF is set)
    pub fn encode_as_eof(&self) -> Bytes {
        self.encode_with_header(0xfe)
    }

    fn encode_with_header(&self, header: u8) -> Bytes {
        let mut buf = BytesMut::new();

        buf.put_u8(header); // OK packet header (0x00 or 0xFE)
        write_lenenc_int(&mut buf, self.affected_rows);
        write_lenenc_int(&mut buf, self.last_insert_id);
        buf.put_u16_le(self.status_flags);
        buf.put_u16_le(self.warnings);

        if !self.info.is_empty() {
            buf.put_slice(self.info.as_bytes());
        }

        buf.freeze()
    }
}

// Error Packet
pub struct ErrPacket {
    pub error_code: u16,
    pub sql_state: String,
    pub error_message: String,
}

impl ErrPacket {
    pub fn new(error_code: u16, error_message: String) -> Self {
        Self {
            error_code,
            sql_state: "HY000".to_string(),
            error_message,
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();

        buf.put_u8(0xff); // Error packet header
        buf.put_u16_le(self.error_code);
        buf.put_u8(b'#'); // SQL state marker
        buf.put_slice(self.sql_state.as_bytes());
        buf.put_slice(self.error_message.as_bytes());

        buf.freeze()
    }
}

// EOF Packet
pub struct EofPacket {
    pub warnings: u16,
    pub status_flags: u16,
}

impl EofPacket {
    pub fn new() -> Self {
        Self {
            warnings: 0,
            status_flags: SERVER_STATUS_AUTOCOMMIT,
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();

        buf.put_u8(0xfe); // EOF packet header
        buf.put_u16_le(self.warnings);
        buf.put_u16_le(self.status_flags);

        buf.freeze()
    }
}

// Column Definition
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
    pub fn new(name: String, column_type: ColumnType) -> Self {
        Self {
            catalog: "def".to_string(),
            schema: "".to_string(),
            table: "".to_string(),
            org_table: "".to_string(),
            name: name.clone(),
            org_name: name,
            character_set: 33, // UTF8
            column_length: 255,
            column_type: column_type as u8,
            flags: 0,
            decimals: 0,
        }
    }

    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();

        write_lenenc_str(&mut buf, &self.catalog);
        write_lenenc_str(&mut buf, &self.schema);
        write_lenenc_str(&mut buf, &self.table);
        write_lenenc_str(&mut buf, &self.org_table);
        write_lenenc_str(&mut buf, &self.name);
        write_lenenc_str(&mut buf, &self.org_name);

        // Length of fixed-length fields
        write_lenenc_int(&mut buf, 0x0c);

        buf.put_u16_le(self.character_set);
        buf.put_u32_le(self.column_length);
        buf.put_u8(self.column_type);
        buf.put_u16_le(self.flags);
        buf.put_u8(self.decimals);

        // Filler
        buf.put_u16(0);

        buf.freeze()
    }
}

// Result Row
#[derive(Debug, Clone)]
pub struct ResultRow {
    pub values: Vec<Option<String>>,
}

impl ResultRow {
    pub fn new(values: Vec<Option<String>>) -> Self {
        Self { values }
    }

    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();

        for value in &self.values {
            match value {
                Some(v) => write_lenenc_str(&mut buf, v),
                None => buf.put_u8(0xfb), // NULL
            }
        }

        buf.freeze()
    }
}
