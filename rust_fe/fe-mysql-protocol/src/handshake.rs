// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! MySQL Handshake Protocol
//!
//! Implements the MySQL handshake sequence:
//! 1. Server sends Initial Handshake packet (v10)
//! 2. Client responds with HandshakeResponse41
//! 3. Server sends OK or ERR packet

use crate::constants::*;
use crate::packet::Packet;
use byteorder::{LittleEndian, WriteBytesExt};
use rand::Rng;
use sha1::{Sha1, Digest};

/// Initial handshake packet sent by server
#[derive(Debug, Clone)]
pub struct InitialHandshake {
    pub protocol_version: u8,
    pub server_version: String,
    pub connection_id: u32,
    pub auth_plugin_data: Vec<u8>,  // 20 bytes for mysql_native_password
    pub capability_flags: u32,
    pub character_set: u8,
    pub status_flags: u16,
    pub auth_plugin_name: String,
}

impl InitialHandshake {
    /// Create new handshake with random salt
    pub fn new(connection_id: u32) -> Self {
        let mut rng = rand::rng();
        let mut auth_plugin_data = vec![0u8; 20];
        rng.fill(&mut auth_plugin_data[..]);

        Self {
            protocol_version: PROTOCOL_VERSION,
            server_version: SERVER_VERSION.to_string(),
            connection_id,
            auth_plugin_data,
            capability_flags: DEFAULT_CAPABILITY_FLAGS,
            character_set: UTF8MB4_GENERAL_CI,
            status_flags: SERVER_STATUS_AUTOCOMMIT,
            auth_plugin_name: "mysql_native_password".to_string(),
        }
    }

    /// Encode to packet payload
    pub fn to_packet(&self, sequence_id: u8) -> Packet {
        let mut payload = Vec::new();

        // Protocol version (1 byte)
        payload.push(self.protocol_version);

        // Server version (null-terminated string)
        payload.extend_from_slice(self.server_version.as_bytes());
        payload.push(0x00);

        // Connection ID (4 bytes)
        payload.write_u32::<LittleEndian>(self.connection_id).unwrap();

        // Auth plugin data part 1 (8 bytes)
        payload.extend_from_slice(&self.auth_plugin_data[0..8]);
        payload.push(0x00); // Filler

        // Capability flags lower 2 bytes
        payload.write_u16::<LittleEndian>((self.capability_flags & 0xFFFF) as u16).unwrap();

        // Character set (1 byte)
        payload.push(self.character_set);

        // Status flags (2 bytes)
        payload.write_u16::<LittleEndian>(self.status_flags).unwrap();

        // Capability flags upper 2 bytes
        payload.write_u16::<LittleEndian>((self.capability_flags >> 16) as u16).unwrap();

        // Auth plugin data length (1 byte)
        payload.push(self.auth_plugin_data.len() as u8);

        // Reserved (10 bytes of 0x00)
        payload.extend_from_slice(&[0u8; 10]);

        // Auth plugin data part 2 (rest of the data)
        payload.extend_from_slice(&self.auth_plugin_data[8..]);
        payload.push(0x00);

        // Auth plugin name (null-terminated string)
        payload.extend_from_slice(self.auth_plugin_name.as_bytes());
        payload.push(0x00);

        Packet::new(sequence_id, payload)
    }
}

/// Handshake response from client (Protocol 4.1)
#[derive(Debug, Clone)]
pub struct HandshakeResponse41 {
    pub capability_flags: u32,
    pub max_packet_size: u32,
    pub character_set: u8,
    pub username: String,
    pub auth_response: Vec<u8>,
    pub database: Option<String>,
    pub auth_plugin_name: Option<String>,
    pub connect_attrs: Vec<(String, String)>,
}

impl HandshakeResponse41 {
    /// Parse from packet payload
    pub fn from_packet(packet: &Packet) -> std::io::Result<Self> {
        use std::io::{Cursor, Read};
        use byteorder::ReadBytesExt;
        use crate::packet::read_lenenc_string;

        let mut cursor = Cursor::new(&packet.payload);

        // Capability flags (4 bytes)
        let capability_flags = cursor.read_u32::<LittleEndian>()?;

        // Max packet size (4 bytes)
        let max_packet_size = cursor.read_u32::<LittleEndian>()?;

        // Character set (1 byte)
        let character_set = cursor.read_u8()?;

        // Reserved (23 bytes of 0x00)
        let mut reserved = [0u8; 23];
        cursor.read_exact(&mut reserved)?;

        // Username (null-terminated string)
        let mut username_bytes = Vec::new();
        loop {
            let byte = cursor.read_u8()?;
            if byte == 0x00 {
                break;
            }
            username_bytes.push(byte);
        }
        let username = String::from_utf8_lossy(&username_bytes).to_string();

        // Auth response
        let auth_response = if capability_flags & CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA != 0 {
            // Length-encoded auth response
            read_lenenc_string(&mut cursor)?
        } else if capability_flags & CLIENT_SECURE_CONNECTION != 0 {
            // Length-prefixed auth response (1 byte length)
            let len = cursor.read_u8()? as usize;
            let mut data = vec![0u8; len];
            cursor.read_exact(&mut data)?;
            data
        } else {
            // Null-terminated auth response
            let mut data = Vec::new();
            loop {
                let byte = cursor.read_u8()?;
                if byte == 0x00 {
                    break;
                }
                data.push(byte);
            }
            data
        };

        // Database (if CLIENT_CONNECT_WITH_DB is set)
        let database = if capability_flags & CLIENT_CONNECT_WITH_DB != 0 {
            let mut db_bytes = Vec::new();
            loop {
                match cursor.read_u8() {
                    Ok(byte) if byte != 0x00 => db_bytes.push(byte),
                    _ => break,
                }
            }
            if !db_bytes.is_empty() {
                Some(String::from_utf8_lossy(&db_bytes).to_string())
            } else {
                None
            }
        } else {
            None
        };

        // Auth plugin name (if CLIENT_PLUGIN_AUTH is set)
        let auth_plugin_name = if capability_flags & CLIENT_PLUGIN_AUTH != 0 {
            let mut plugin_bytes = Vec::new();
            loop {
                match cursor.read_u8() {
                    Ok(byte) if byte != 0x00 => plugin_bytes.push(byte),
                    _ => break,
                }
            }
            if !plugin_bytes.is_empty() {
                Some(String::from_utf8_lossy(&plugin_bytes).to_string())
            } else {
                None
            }
        } else {
            None
        };

        // Connect attributes (if CLIENT_CONNECT_ATTRS is set)
        let connect_attrs = Vec::new(); // TODO: Parse connect attrs if needed

        Ok(Self {
            capability_flags,
            max_packet_size,
            character_set,
            username,
            auth_response,
            database,
            auth_plugin_name,
            connect_attrs,
        })
    }
}

/// Verify password using mysql_native_password algorithm
pub fn verify_native_password(
    password: &str,
    auth_response: &[u8],
    auth_plugin_data: &[u8],
) -> bool {
    if password.is_empty() {
        return auth_response.is_empty();
    }

    // MySQL native password algorithm:
    // SHA1(password) XOR SHA1(auth_data + SHA1(SHA1(password)))

    // Step 1: SHA1(password)
    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    let sha1_pass = hasher.finalize();

    // Step 2: SHA1(SHA1(password))
    let mut hasher = Sha1::new();
    hasher.update(&sha1_pass);
    let sha1_sha1_pass = hasher.finalize();

    // Step 3: SHA1(auth_data + SHA1(SHA1(password)))
    let mut hasher = Sha1::new();
    hasher.update(auth_plugin_data);
    hasher.update(&sha1_sha1_pass);
    let hash = hasher.finalize();

    // Step 4: SHA1(password) XOR SHA1(auth_data + SHA1(SHA1(password)))
    let expected: Vec<u8> = sha1_pass.iter()
        .zip(hash.iter())
        .map(|(a, b)| a ^ b)
        .collect();

    expected == auth_response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_handshake_packet() {
        let handshake = InitialHandshake::new(12345);
        let packet = handshake.to_packet(0);

        assert_eq!(packet.sequence_id, 0);
        assert!(!packet.payload.is_empty());

        // Check protocol version
        assert_eq!(packet.payload[0], PROTOCOL_VERSION);

        // Check server version is present
        assert!(packet.payload.len() > SERVER_VERSION.len());
    }

    #[test]
    fn test_handshake_auth_plugin_data() {
        let handshake = InitialHandshake::new(1);
        assert_eq!(handshake.auth_plugin_data.len(), 20);

        // Should be random, so two instances should differ
        let handshake2 = InitialHandshake::new(2);
        assert_ne!(handshake.auth_plugin_data, handshake2.auth_plugin_data);
    }

    #[test]
    fn test_native_password_empty() {
        let auth_data = vec![0u8; 20];
        let empty_response = vec![];

        // Empty password should match empty response
        assert!(verify_native_password("", &empty_response, &auth_data));

        // Empty password should NOT match non-empty response
        assert!(!verify_native_password("", &[1, 2, 3], &auth_data));
    }

    #[test]
    fn test_native_password_algorithm() {
        // Test with known values
        let password = "root";
        let auth_data = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
            0x11, 0x12, 0x13, 0x14
        ];

        // Compute expected auth response
        let mut hasher = Sha1::new();
        hasher.update(password.as_bytes());
        let sha1_pass = hasher.finalize();

        let mut hasher = Sha1::new();
        hasher.update(&sha1_pass);
        let sha1_sha1_pass = hasher.finalize();

        let mut hasher = Sha1::new();
        hasher.update(&auth_data);
        hasher.update(&sha1_sha1_pass);
        let hash = hasher.finalize();

        let auth_response: Vec<u8> = sha1_pass.iter()
            .zip(hash.iter())
            .map(|(a, b)| a ^ b)
            .collect();

        // Verify it matches
        assert!(verify_native_password(password, &auth_response, &auth_data));

        // Verify wrong password doesn't match
        assert!(!verify_native_password("wrong", &auth_response, &auth_data));
    }

    #[test]
    fn test_handshake_response_parsing() {
        // Create a minimal handshake response packet
        let mut payload = Vec::new();

        // Capability flags
        payload.write_u32::<LittleEndian>(CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION).unwrap();

        // Max packet size
        payload.write_u32::<LittleEndian>(0x01000000).unwrap();

        // Character set
        payload.push(UTF8MB4_GENERAL_CI);

        // Reserved (23 bytes)
        payload.extend_from_slice(&[0u8; 23]);

        // Username (null-terminated)
        payload.extend_from_slice(b"root");
        payload.push(0x00);

        // Auth response (length-prefixed for CLIENT_SECURE_CONNECTION)
        payload.push(20); // Length
        payload.extend_from_slice(&[0u8; 20]); // Dummy auth data

        let packet = Packet::new(1, payload);
        let response = HandshakeResponse41::from_packet(&packet).unwrap();

        assert_eq!(response.username, "root");
        assert_eq!(response.auth_response.len(), 20);
        assert_eq!(response.character_set, UTF8MB4_GENERAL_CI);
    }
}
