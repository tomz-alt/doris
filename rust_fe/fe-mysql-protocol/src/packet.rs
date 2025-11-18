// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! MySQL Protocol Packet Framing
//!
//! MySQL wire protocol uses this packet format:
//! ```text
//! [3 bytes: payload length]
//! [1 byte: sequence number]
//! [N bytes: payload]
//! ```

use std::io::{self, Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

/// MySQL packet header (4 bytes)
#[derive(Debug, Clone)]
pub struct PacketHeader {
    pub payload_length: u32,  // Actually 3 bytes (u24)
    pub sequence_id: u8,
}

impl PacketHeader {
    pub const SIZE: usize = 4;
    pub const MAX_PAYLOAD_LEN: u32 = 0xFFFFFF; // 16MB - 1

    pub fn new(payload_length: u32, sequence_id: u8) -> Self {
        assert!(payload_length <= Self::MAX_PAYLOAD_LEN);
        Self { payload_length, sequence_id }
    }

    /// Read packet header from stream
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        // Read 3-byte payload length (little-endian)
        let mut len_bytes = [0u8; 3];
        reader.read_exact(&mut len_bytes)?;
        let payload_length = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], 0]);

        // Read 1-byte sequence ID
        let sequence_id = reader.read_u8()?;

        Ok(Self::new(payload_length, sequence_id))
    }

    /// Write packet header to stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Write 3-byte payload length (little-endian)
        let len_bytes = self.payload_length.to_le_bytes();
        writer.write_all(&len_bytes[0..3])?;

        // Write 1-byte sequence ID
        writer.write_u8(self.sequence_id)?;

        Ok(())
    }
}

/// MySQL packet (header + payload)
#[derive(Debug, Clone)]
pub struct Packet {
    pub sequence_id: u8,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn new(sequence_id: u8, payload: Vec<u8>) -> Self {
        Self { sequence_id, payload }
    }

    /// Read a complete packet from stream
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let header = PacketHeader::read_from(reader)?;

        let mut payload = vec![0u8; header.payload_length as usize];
        reader.read_exact(&mut payload)?;

        Ok(Self::new(header.sequence_id, payload))
    }

    /// Write a complete packet to stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Handle packets larger than 16MB - 1 by splitting
        if self.payload.len() > PacketHeader::MAX_PAYLOAD_LEN as usize {
            return self.write_large_packet(writer);
        }

        let header = PacketHeader::new(self.payload.len() as u32, self.sequence_id);
        header.write_to(writer)?;
        writer.write_all(&self.payload)?;
        writer.flush()?;

        Ok(())
    }

    /// Write packet larger than 16MB by splitting into multiple packets
    fn write_large_packet<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut remaining = &self.payload[..];
        let mut seq = self.sequence_id;

        while !remaining.is_empty() {
            let chunk_size = std::cmp::min(remaining.len(), PacketHeader::MAX_PAYLOAD_LEN as usize);
            let (chunk, rest) = remaining.split_at(chunk_size);

            let header = PacketHeader::new(chunk_size as u32, seq);
            header.write_to(writer)?;
            writer.write_all(chunk)?;

            remaining = rest;
            seq = seq.wrapping_add(1);
        }

        writer.flush()?;
        Ok(())
    }

    /// Create OK packet
    pub fn ok(affected_rows: u64, last_insert_id: u64, status_flags: u16, warnings: u16, sequence_id: u8) -> Self {
        let mut payload = Vec::new();
        payload.push(0x00); // OK packet header

        // Affected rows (length-encoded integer)
        write_lenenc_int(&mut payload, affected_rows);

        // Last insert ID (length-encoded integer)
        write_lenenc_int(&mut payload, last_insert_id);

        // Status flags
        payload.write_u16::<LittleEndian>(status_flags).unwrap();

        // Warnings
        payload.write_u16::<LittleEndian>(warnings).unwrap();

        Self::new(sequence_id, payload)
    }

    /// Create ERR packet
    pub fn err(error_code: u16, sql_state: &str, error_message: &str, sequence_id: u8) -> Self {
        let mut payload = Vec::new();
        payload.push(0xFF); // ERR packet header

        // Error code
        payload.write_u16::<LittleEndian>(error_code).unwrap();

        // SQL state marker + state (6 bytes total)
        payload.push(b'#');
        payload.extend_from_slice(sql_state.as_bytes());

        // Error message
        payload.extend_from_slice(error_message.as_bytes());

        Self::new(sequence_id, payload)
    }

    /// Create EOF packet
    pub fn eof(warnings: u16, status_flags: u16, sequence_id: u8) -> Self {
        let mut payload = Vec::new();
        payload.push(0xFE); // EOF packet header

        // Warnings
        payload.write_u16::<LittleEndian>(warnings).unwrap();

        // Status flags
        payload.write_u16::<LittleEndian>(status_flags).unwrap();

        Self::new(sequence_id, payload)
    }
}

/// Write length-encoded integer (MySQL protocol)
pub fn write_lenenc_int(buf: &mut Vec<u8>, value: u64) {
    if value < 251 {
        buf.push(value as u8);
    } else if value < 0x10000 {
        buf.push(0xFC);
        buf.write_u16::<LittleEndian>(value as u16).unwrap();
    } else if value < 0x1000000 {
        buf.push(0xFD);
        buf.write_u24::<LittleEndian>(value as u32).unwrap();
    } else {
        buf.push(0xFE);
        buf.write_u64::<LittleEndian>(value).unwrap();
    }
}

/// Read length-encoded integer (MySQL protocol)
pub fn read_lenenc_int<R: Read>(reader: &mut R) -> io::Result<u64> {
    let first_byte = reader.read_u8()?;
    match first_byte {
        0..=250 => Ok(first_byte as u64),
        0xFC => Ok(reader.read_u16::<LittleEndian>()? as u64),
        0xFD => Ok(reader.read_u24::<LittleEndian>()? as u64),
        0xFE => reader.read_u64::<LittleEndian>(),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid length-encoded integer")),
    }
}

/// Read length-encoded string (MySQL protocol)
pub fn read_lenenc_string<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
    let len = read_lenenc_int(reader)?;
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

/// Write length-encoded string (MySQL protocol)
pub fn write_lenenc_string(buf: &mut Vec<u8>, s: &[u8]) {
    write_lenenc_int(buf, s.len() as u64);
    buf.extend_from_slice(s);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_packet_header_read_write() {
        let header = PacketHeader::new(100, 5);

        let mut buf = Vec::new();
        header.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 4);

        let mut cursor = Cursor::new(buf);
        let read_header = PacketHeader::read_from(&mut cursor).unwrap();

        assert_eq!(read_header.payload_length, 100);
        assert_eq!(read_header.sequence_id, 5);
    }

    #[test]
    fn test_packet_read_write() {
        let payload = b"SELECT * FROM users".to_vec();
        let packet = Packet::new(1, payload.clone());

        let mut buf = Vec::new();
        packet.write_to(&mut buf).unwrap();

        let mut cursor = Cursor::new(buf);
        let read_packet = Packet::read_from(&mut cursor).unwrap();

        assert_eq!(read_packet.sequence_id, 1);
        assert_eq!(read_packet.payload, payload);
    }

    #[test]
    fn test_ok_packet() {
        let packet = Packet::ok(1, 0, 0x0002, 0, 1);
        assert_eq!(packet.payload[0], 0x00); // OK header
        assert_eq!(packet.sequence_id, 1);
    }

    #[test]
    fn test_err_packet() {
        let packet = Packet::err(1064, "42000", "Syntax error", 1);
        assert_eq!(packet.payload[0], 0xFF); // ERR header
        assert_eq!(packet.sequence_id, 1);
    }

    #[test]
    fn test_lenenc_int() {
        let mut buf = Vec::new();

        // < 251: 1 byte
        write_lenenc_int(&mut buf, 250);
        assert_eq!(buf, vec![250]);

        buf.clear();

        // < 2^16: 3 bytes (0xFC + 2 bytes)
        write_lenenc_int(&mut buf, 1000);
        assert_eq!(buf.len(), 3);
        assert_eq!(buf[0], 0xFC);
    }

    #[test]
    fn test_lenenc_string() {
        let mut buf = Vec::new();
        let test_str = b"hello";

        write_lenenc_string(&mut buf, test_str);

        let mut cursor = Cursor::new(buf);
        let read_str = read_lenenc_string(&mut cursor).unwrap();

        assert_eq!(read_str, test_str);
    }
}
