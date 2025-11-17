#!/usr/bin/env python3
"""
Extract Thrift payloads from pcap files.
Extracts TCP payload data sent to BE port 8060.
"""

import sys
import subprocess
from pathlib import Path


def extract_with_tshark(pcap_file, output_file, port=8060):
    """Extract TCP payload using tshark."""
    print(f"Extracting payloads from {pcap_file} (port {port})...")

    # Use tshark to extract TCP payload data going to BE port
    cmd = [
        "tshark",
        "-r", str(pcap_file),
        "-Y", f"tcp.dstport=={port} and tcp.len>0",
        "-T", "fields",
        "-e", "tcp.payload"
    ]

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        hex_data = result.stdout.strip()

        if not hex_data:
            print(f"WARNING: No TCP payload found in {pcap_file}")
            return False

        # Convert hex string to binary
        # Remove colons and convert
        hex_clean = hex_data.replace(":", "").replace("\n", "")
        binary_data = bytes.fromhex(hex_clean)

        # Write to file
        with open(output_file, "wb") as f:
            f.write(binary_data)

        print(f"✓ Extracted {len(binary_data)} bytes to {output_file}")

        # Show first 64 bytes
        print("\nFirst 64 bytes (hex):")
        subprocess.run(["xxd", "-g", "1", "-l", "64", str(output_file)])

        return True

    except subprocess.CalledProcessError as e:
        print(f"ERROR: tshark failed: {e}")
        print(f"stderr: {e.stderr}")
        return False
    except Exception as e:
        print(f"ERROR: {e}")
        return False


def main():
    if len(sys.argv) < 3:
        print("Usage: extract_thrift_payload.py <pcap_file> <output_file> [port]")
        print("Example: extract_thrift_payload.py capture.pcap payload.bin 8060")
        sys.exit(1)

    pcap_file = Path(sys.argv[1])
    output_file = Path(sys.argv[2])
    port = int(sys.argv[3]) if len(sys.argv) > 3 else 8060

    if not pcap_file.exists():
        print(f"ERROR: {pcap_file} not found")
        sys.exit(1)

    success = extract_with_tshark(pcap_file, output_file, port)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
