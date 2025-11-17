#!/usr/bin/env python3
"""
Simple pcap parser to extract TCP payloads - no external dependencies.
Works with standard pcap format (not pcapng).
"""

import struct
import sys
from pathlib import Path


def parse_pcap(pcap_file, dst_port=8060):
    """Parse pcap file and extract TCP payloads to destination port."""

    with open(pcap_file, 'rb') as f:
        # Read pcap global header
        magic = f.read(4)

        # Check magic number
        if magic == b'\xa1\xb2\xc3\xd4':
            # Standard pcap (microsecond resolution)
            byte_order = '>'
        elif magic == b'\xd4\xc3\xb2\xa1':
            byte_order = '<'
        else:
            print(f"Unknown magic: {magic.hex()}")
            return []

        # Skip rest of global header (20 bytes)
        f.read(20)

        payloads = []
        packet_num = 0

        while True:
            # Read packet header (16 bytes)
            pkt_header = f.read(16)
            if len(pkt_header) < 16:
                break

            # Parse packet header
            ts_sec, ts_usec, incl_len, orig_len = struct.unpack(f'{byte_order}IIII', pkt_header)

            # Read packet data
            pkt_data = f.read(incl_len)
            if len(pkt_data) < incl_len:
                break

            packet_num += 1

            try:
                # Detect link layer type
                # LINUX_SLL2 has 20 byte header
                # Standard Ethernet has 14 byte header

                # Try to detect LINUX_SLL2 (starts with 0x0a 0x00 for version 2)
                if len(pkt_data) >= 20 and pkt_data[0:2] == b'\x0a\x00':
                    # LINUX_SLL2 format
                    # Protocol type is at offset 0 (2 bytes, big endian)
                    # EtherType is at offset 16 (2 bytes, big endian)
                    eth_type = struct.unpack('!H', pkt_data[16:18])[0]
                    ip_start = 20
                elif len(pkt_data) >= 14:
                    # Standard Ethernet
                    eth_type = struct.unpack('!H', pkt_data[12:14])[0]
                    ip_start = 14
                else:
                    continue

                # Check for IPv4 (0x0800)
                if eth_type != 0x0800:
                    continue

                # Parse IP header
                if len(pkt_data) < ip_start + 20:
                    continue

                ip_version_ihl = pkt_data[ip_start]
                ip_ihl = (ip_version_ihl & 0x0F) * 4
                ip_protocol = pkt_data[ip_start + 9]

                # Check for TCP (6)
                if ip_protocol != 6:
                    continue

                # Parse TCP header
                tcp_start = ip_start + ip_ihl
                if len(pkt_data) < tcp_start + 20:
                    continue

                tcp_src_port = struct.unpack('!H', pkt_data[tcp_start:tcp_start+2])[0]
                tcp_dst_port = struct.unpack('!H', pkt_data[tcp_start+2:tcp_start+4])[0]
                tcp_data_offset = (pkt_data[tcp_start + 12] >> 4) * 4

                # Check destination port
                if tcp_dst_port != dst_port:
                    continue

                # Extract TCP payload
                tcp_payload_start = tcp_start + tcp_data_offset
                tcp_payload = pkt_data[tcp_payload_start:]

                if len(tcp_payload) > 0:
                    print(f"Packet {packet_num}: {tcp_src_port} -> {tcp_dst_port}, {len(tcp_payload)} bytes payload")
                    payloads.append(tcp_payload)

            except Exception as e:
                print(f"Error parsing packet {packet_num}: {e}")
                continue

        return payloads


def main():
    if len(sys.argv) < 3:
        print("Usage: simple_pcap_extract.py <pcap_file> <output_file> [dst_port]")
        sys.exit(1)

    pcap_file = Path(sys.argv[1])
    output_file = Path(sys.argv[2])
    dst_port = int(sys.argv[3]) if len(sys.argv) > 3 else 8060

    if not pcap_file.exists():
        print(f"ERROR: {pcap_file} not found")
        sys.exit(1)

    print(f"Extracting TCP payloads from {pcap_file} (dst port {dst_port})...")
    payloads = parse_pcap(pcap_file, dst_port)

    if not payloads:
        print("No TCP payloads found!")
        sys.exit(1)

    # Concatenate all payloads
    combined = b''.join(payloads)

    # Write to file
    with open(output_file, 'wb') as f:
        f.write(combined)

    print(f"\n✓ Extracted {len(payloads)} TCP segments ({len(combined)} total bytes)")
    print(f"  Output: {output_file}")

    # Show first 128 bytes
    print("\nFirst 128 bytes (hex):")
    import subprocess
    subprocess.run(["xxd", "-g", "1", "-l", "128", str(output_file)])


if __name__ == "__main__":
    main()
