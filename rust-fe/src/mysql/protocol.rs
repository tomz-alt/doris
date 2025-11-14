use bytes::{Buf, BufMut, Bytes, BytesMut};

// MySQL Command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Query = 0x03,
    InitDb = 0x02,
    Quit = 0x01,
    Ping = 0x0e,
    FieldList = 0x04,
    StmtPrepare = 0x16,
    StmtExecute = 0x17,
    StmtClose = 0x19,
    Unknown(u8),
}

impl From<u8> for Command {
    fn from(byte: u8) -> Self {
        match byte {
            0x01 => Command::Quit,
            0x02 => Command::InitDb,
            0x03 => Command::Query,
            0x0e => Command::Ping,
            0x04 => Command::FieldList,
            0x16 => Command::StmtPrepare,
            0x17 => Command::StmtExecute,
            0x19 => Command::StmtClose,
            b => Command::Unknown(b),
        }
    }
}

// MySQL Capability Flags
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

// Server capability flags
pub fn server_capabilities() -> u32 {
    CLIENT_LONG_PASSWORD
        | CLIENT_FOUND_ROWS
        | CLIENT_LONG_FLAG
        | CLIENT_CONNECT_WITH_DB
        | CLIENT_ODBC
        | CLIENT_IGNORE_SPACE
        | CLIENT_PROTOCOL_41
        | CLIENT_INTERACTIVE
        | CLIENT_IGNORE_SIGPIPE
        | CLIENT_TRANSACTIONS
        | CLIENT_SECURE_CONNECTION
        | CLIENT_MULTI_STATEMENTS
        | CLIENT_MULTI_RESULTS
        | CLIENT_PS_MULTI_RESULTS
        | CLIENT_PLUGIN_AUTH
        | CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA
        | CLIENT_DEPRECATE_EOF
}

// Character set
pub const UTF8MB4_GENERAL_CI: u8 = 45;

// Status flags
pub const SERVER_STATUS_AUTOCOMMIT: u16 = 0x0002;

// Column types
#[derive(Debug, Clone, Copy)]
pub enum ColumnType {
    Decimal = 0x00,
    Tiny = 0x01,
    Short = 0x02,
    Long = 0x03,
    Float = 0x04,
    Double = 0x05,
    Null = 0x06,
    Timestamp = 0x07,
    LongLong = 0x08,
    Int24 = 0x09,
    Date = 0x0a,
    Time = 0x0b,
    DateTime = 0x0c,
    Year = 0x0d,
    VarChar = 0x0f,
    Bit = 0x10,
    Json = 0xf5,
    NewDecimal = 0xf6,
    Enum = 0xf7,
    Set = 0xf8,
    TinyBlob = 0xf9,
    MediumBlob = 0xfa,
    LongBlob = 0xfb,
    Blob = 0xfc,
    VarString = 0xfd,
    String = 0xfe,
    Geometry = 0xff,
}

// Helper functions for length-encoded integers
pub fn write_lenenc_int(buf: &mut BytesMut, value: u64) {
    if value < 251 {
        buf.put_u8(value as u8);
    } else if value < 65536 {
        buf.put_u8(0xfc);
        buf.put_u16_le(value as u16);
    } else if value < 16777216 {
        buf.put_u8(0xfd);
        buf.put_uint_le(value, 3);
    } else {
        buf.put_u8(0xfe);
        buf.put_u64_le(value);
    }
}

pub fn write_lenenc_str(buf: &mut BytesMut, s: &str) {
    write_lenenc_int(buf, s.len() as u64);
    buf.put_slice(s.as_bytes());
}

pub fn write_null_terminated_str(buf: &mut BytesMut, s: &str) {
    buf.put_slice(s.as_bytes());
    buf.put_u8(0);
}

pub fn read_lenenc_int(buf: &mut Bytes) -> Option<u64> {
    if buf.is_empty() {
        return None;
    }

    let first = buf.get_u8();
    match first {
        0xfb => Some(0), // NULL
        0xfc => {
            if buf.remaining() < 2 {
                return None;
            }
            Some(buf.get_u16_le() as u64)
        }
        0xfd => {
            if buf.remaining() < 3 {
                return None;
            }
            Some(buf.get_uint_le(3))
        }
        0xfe => {
            if buf.remaining() < 8 {
                return None;
            }
            Some(buf.get_u64_le())
        }
        _ => Some(first as u64),
    }
}

pub fn read_null_terminated_string(buf: &mut Bytes) -> Option<String> {
    let mut result = Vec::new();
    while buf.has_remaining() {
        let byte = buf.get_u8();
        if byte == 0 {
            return String::from_utf8(result).ok();
        }
        result.push(byte);
    }
    None
}

pub fn read_string_to_end(buf: &mut Bytes) -> String {
    let bytes = buf.copy_to_bytes(buf.remaining());
    String::from_utf8_lossy(&bytes).to_string()
}

// Authentication
pub fn scramble_password(password: &str, salt: &[u8]) -> Vec<u8> {
    use sha1::{Sha1, Digest};

    // SHA1(password)
    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    let hash1 = hasher.finalize();

    // SHA1(SHA1(password))
    let mut hasher = Sha1::new();
    hasher.update(&hash1);
    let hash2 = hasher.finalize();

    // SHA1(salt + SHA1(SHA1(password)))
    let mut hasher = Sha1::new();
    hasher.update(salt);
    hasher.update(&hash2);
    let hash3 = hasher.finalize();

    // XOR hash1 and hash3
    hash1.iter()
        .zip(hash3.iter())
        .map(|(a, b)| a ^ b)
        .collect()
}
