#!/usr/bin/env python3
"""
Extract TCP payloads from pcap files without requiring tshark.
Uses dpkt library for parsing pcap files.
"""

import sys
from pathlib import Path

try:
    import dpkt
except ImportError:
    print("ERROR: dpkt library not found. Installing...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "dpkt"])
    import dpkt


def extract_tcp_payloads(pcap_file, output_file, dst_port=8060):
    """Extract TCP payload data sent to specified destination port."""
    print(f"Extracting TCP payloads from {pcap_file} (dst port {dst_port})...")

    all_payloads = []
    packet_count = 0
    payload_count = 0

    try:
        with open(pcap_file, 'rb') as f:
            try:
                pcap = dpkt.pcap.Reader(f)
            except ValueError:
                # Try pcapng format
                f.seek(0)
                pcap = dpkt.pcapng.Reader(f)

            for timestamp, buf in pcap:
                packet_count += 1

                try:
                    # Parse Ethernet frame
                    eth = dpkt.ethernet.Ethernet(buf)

                    # Check if it's IP
                    if not isinstance(eth.data, dpkt.ip.IP):
                        continue

                    ip = eth.data

                    # Check if it's TCP
                    if not isinstance(ip.data, dpkt.tcp.TCP):
                        continue

                    tcp = ip.data

                    # Check if destination port matches
                    if tcp.dport == dst_port and len(tcp.data) > 0:
                        all_payloads.append(tcp.data)
                        payload_count += 1
                        print(f"  Packet {packet_count}: Found {len(tcp.data)} bytes of TCP payload")

                except (dpkt.dpkt.NeedData, dpkt.dpkt.UnpackError) as e:
                    continue

    except Exception as e:
        print(f"ERROR reading pcap: {e}")
        return False

    if not all_payloads:
        print(f"WARNING: No TCP payloads found for port {dst_port}")
        return False

    # Concatenate all payloads
    combined_payload = b''.join(all_payloads)

    # Write to file
    with open(output_file, 'wb') as f:
        f.write(combined_payload)

    print(f"\n✓ Extracted {payload_count} TCP segments ({len(combined_payload)} total bytes)")
    print(f"  Output: {output_file}")

    # Show first 128 bytes using xxd
    print("\nFirst 128 bytes (hex):")
    import subprocess
    subprocess.run(["xxd", "-g", "1", "-l", "128", str(output_file)])

    return True


def main():
    if len(sys.argv) < 3:
        print("Usage: extract_tcp_payload.py <pcap_file> <output_file> [dst_port]")
        print("Example: extract_tcp_payload.py capture.pcap payload.bin 8060")
        sys.exit(1)

    pcap_file = Path(sys.argv[1])
    output_file = Path(sys.argv[2])
    dst_port = int(sys.argv[3]) if len(sys.argv) > 3 else 8060

    if not pcap_file.exists():
        print(f"ERROR: {pcap_file} not found")
        sys.exit(1)

    success = extract_tcp_payloads(pcap_file, output_file, dst_port)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
