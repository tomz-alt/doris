#!/usr/bin/env python3
"""
TPC-H Benchmark Runner - Wrapper for ClickBench-style benchmark
"""
import sys
import os

# Add script directory to path
sys.path.insert(0, os.path.dirname(__file__))

from benchmark_clickbench import main, BenchmarkRunner
import argparse

if __name__ == '__main__':
    # Parse arguments with TPC-H defaults
    parser = argparse.ArgumentParser(
        description='TPC-H Benchmark Runner with ClickBench-style Visualization'
    )

    parser.add_argument('-s', '--scale', type=int, default=1,
                        help='TPC-H scale factor (default: 1)')
    parser.add_argument('-r', '--rounds', type=int, default=3,
                        help='Number of benchmark rounds per query (default: 3)')
    parser.add_argument('-w', '--warmup', type=int, default=1,
                        help='Number of warmup rounds (default: 1)')

    parser.add_argument('--java-host', default='127.0.0.1',
                        help='Java FE host (default: 127.0.0.1)')
    parser.add_argument('--java-port', type=int, default=9030,
                        help='Java FE MySQL port (default: 9030)')

    parser.add_argument('--rust-host', default='127.0.0.1',
                        help='Rust FE host (default: 127.0.0.1)')
    parser.add_argument('--rust-port', type=int, default=9031,
                        help='Rust FE MySQL port (default: 9031)')

    parser.add_argument('--queries-dir', default='scripts/tpch/queries',
                        help='Directory containing TPC-H query SQL files')

    parser.add_argument('--output-json', default='tpch_results.json',
                        help='Output JSON file')
    parser.add_argument('--output-html', default='tpch_results.html',
                        help='Output HTML file')

    args = parser.parse_args()
    args.benchmark_name = 'TPC-H'

    # Run benchmark
    runner = BenchmarkRunner(args)
    runner.run_benchmark()
