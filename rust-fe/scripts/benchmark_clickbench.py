#!/usr/bin/env python3
"""
TPC-H/TPC-DS Benchmark Runner with ClickBench-style Visualization
Runs queries against Java FE and Rust FE, generates ClickBench-style HTML report
"""

import argparse
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Tuple
import statistics

try:
    import mysql.connector
except ImportError:
    print("Error: mysql-connector-python not installed")
    print("Install with: pip3 install mysql-connector-python")
    sys.exit(1)


class BenchmarkRunner:
    def __init__(self, args):
        self.args = args
        self.benchmark_name = args.benchmark_name if hasattr(args, 'benchmark_name') else 'TPC-H'
        self.results = {
            'metadata': {
                'benchmark': self.benchmark_name,
                'scale_factor': args.scale,
                'rounds': args.rounds,
                'timestamp': datetime.now().isoformat(),
                'java_fe': f"{args.java_host}:{args.java_port}",
                'rust_fe': f"{args.rust_host}:{args.rust_port}",
            },
            'queries': {}
        }
        self.queries_dir = Path(args.queries_dir)

    def get_connection(self, host, port):
        """Create MySQL connection"""
        try:
            dbname = f"tpch_sf{self.args.scale}" if self.benchmark_name == 'TPC-H' else f"tpcds_sf{self.args.scale}"
            conn = mysql.connector.connect(
                host=host,
                port=port,
                user='root',
                password='',
                database=dbname,
                connect_timeout=30
            )
            return conn
        except Exception as e:
            print(f"Error connecting to {host}:{port}: {e}")
            return None

    def run_query(self, conn, query: str, warmup: bool = False) -> Tuple[float, bool]:
        """Run a single query and return execution time in seconds"""
        try:
            cursor = conn.cursor()
            start_time = time.time()
            cursor.execute(query)

            # Fetch all results to ensure query completes
            if not warmup:
                _ = cursor.fetchall()

            end_time = time.time()
            cursor.close()

            elapsed = end_time - start_time
            return elapsed, True
        except Exception as e:
            if not warmup:
                print(f"  Query failed: {e}")
            return 0.0, False

    def load_query(self, query_file: Path) -> str:
        """Load query from file"""
        with open(query_file, 'r') as f:
            return f.read()

    def run_benchmark(self):
        """Run benchmark for all queries"""

        # Find all query files
        query_files = sorted(self.queries_dir.glob('q*.sql'))

        if not query_files:
            print(f"Error: No query files found in {self.queries_dir}")
            sys.exit(1)

        print("=" * 60)
        print(f"{self.benchmark_name} Benchmark (SF{self.args.scale})")
        print("=" * 60)
        print(f"Java FE:  {self.args.java_host}:{self.args.java_port}")
        print(f"Rust FE:  {self.args.rust_host}:{self.args.rust_port}")
        print(f"Rounds:   {self.args.rounds}")
        print(f"Queries:  {len(query_files)}")
        print("=" * 60)
        print()

        # Connect to both FEs
        java_conn = self.get_connection(self.args.java_host, self.args.java_port)
        rust_conn = self.get_connection(self.args.rust_host, self.args.rust_port)

        if not java_conn:
            print("Error: Could not connect to Java FE")
            sys.exit(1)

        if not rust_conn:
            print("Error: Could not connect to Rust FE")
            sys.exit(1)

        # Run benchmarks
        for query_file in query_files:
            query_name = query_file.stem.upper()
            print(f"\n{query_name}:")
            print("-" * 60)

            query_sql = self.load_query(query_file)

            # Warmup runs
            if self.args.warmup > 0:
                print(f"  Warmup ({self.args.warmup} rounds)...")
                for i in range(self.args.warmup):
                    self.run_query(java_conn, query_sql, warmup=True)
                    self.run_query(rust_conn, query_sql, warmup=True)

            # Actual benchmark runs
            java_times = []
            rust_times = []

            for round_num in range(1, self.args.rounds + 1):
                print(f"  Round {round_num}/{self.args.rounds}:")

                # Run on Java FE
                java_time, java_success = self.run_query(java_conn, query_sql)
                if java_success:
                    java_times.append(java_time)
                    print(f"    Java FE: {java_time:.3f}s")
                else:
                    print(f"    Java FE: FAILED")

                # Brief pause between queries
                time.sleep(0.5)

                # Run on Rust FE
                rust_time, rust_success = self.run_query(rust_conn, query_sql)
                if rust_success:
                    rust_times.append(rust_time)
                    print(f"    Rust FE: {rust_time:.3f}s")
                else:
                    print(f"    Rust FE: FAILED")

                # Show speedup for this round
                if java_success and rust_success and java_time > 0:
                    speedup = java_time / rust_time
                    improvement = ((java_time - rust_time) / java_time) * 100
                    print(f"    Speedup: {speedup:.2f}x ({improvement:+.1f}%)")

            # Calculate statistics
            if java_times and rust_times:
                self.results['queries'][query_name] = {
                    'java_fe': {
                        'times': java_times,
                        'min': min(java_times),
                        'max': max(java_times),
                        'mean': statistics.mean(java_times),
                        'median': statistics.median(java_times),
                        'stdev': statistics.stdev(java_times) if len(java_times) > 1 else 0
                    },
                    'rust_fe': {
                        'times': rust_times,
                        'min': min(rust_times),
                        'max': max(rust_times),
                        'mean': statistics.mean(rust_times),
                        'median': statistics.median(rust_times),
                        'stdev': statistics.stdev(rust_times) if len(rust_times) > 1 else 0
                    },
                    'speedup': {
                        'mean': statistics.mean(java_times) / statistics.mean(rust_times),
                        'median': statistics.median(java_times) / statistics.median(rust_times),
                        'best': min(java_times) / min(rust_times)
                    }
                }

                print(f"\n  Summary:")
                print(f"    Java FE mean: {statistics.mean(java_times):.3f}s ± {statistics.stdev(java_times) if len(java_times) > 1 else 0:.3f}s")
                print(f"    Rust FE mean: {statistics.mean(rust_times):.3f}s ± {statistics.stdev(rust_times) if len(rust_times) > 1 else 0:.3f}s")
                print(f"    Mean speedup: {self.results['queries'][query_name]['speedup']['mean']:.2f}x")

        # Close connections
        java_conn.close()
        rust_conn.close()

        # Print overall summary
        self.print_summary()

        # Save results
        self.save_results()

        # Generate HTML report
        self.generate_html()

    def print_summary(self):
        """Print overall benchmark summary"""
        print("\n" + "=" * 60)
        print("BENCHMARK SUMMARY")
        print("=" * 60)

        all_speedups = []

        print(f"\n{'Query':<8} {'Java FE (mean)':<15} {'Rust FE (mean)':<15} {'Speedup':<10}")
        print("-" * 60)

        for query_name in sorted(self.results['queries'].keys()):
            data = self.results['queries'][query_name]
            java_mean = data['java_fe']['mean']
            rust_mean = data['rust_fe']['mean']
            speedup = data['speedup']['mean']
            all_speedups.append(speedup)

            print(f"{query_name:<8} {java_mean:>10.3f}s     {rust_mean:>10.3f}s     {speedup:>6.2f}x")

        print("-" * 60)

        if all_speedups:
            geometric_mean = statistics.geometric_mean(all_speedups)
            print(f"\n{'Overall Geometric Mean Speedup:':<45} {geometric_mean:.2f}x")
            print(f"{'Average Speedup:':<45} {statistics.mean(all_speedups):.2f}x")
            print(f"{'Median Speedup:':<45} {statistics.median(all_speedups):.2f}x")
            print(f"{'Best Speedup:':<45} {max(all_speedups):.2f}x")

            # Store overall stats
            self.results['overall'] = {
                'geometric_mean': geometric_mean,
                'arithmetic_mean': statistics.mean(all_speedups),
                'median': statistics.median(all_speedups),
                'best': max(all_speedups),
                'worst': min(all_speedups)
            }

        print("=" * 60)

    def save_results(self):
        """Save results to JSON file"""
        output_file = self.args.output_json

        with open(output_file, 'w') as f:
            json.dump(self.results, f, indent=2)

        print(f"\nResults saved to: {output_file}")

    def generate_html(self):
        """Generate ClickBench-style HTML report"""
        output_file = self.args.output_html

        html_content = self.create_clickbench_html()

        with open(output_file, 'w') as f:
            f.write(html_content)

        print(f"HTML report saved to: {output_file}")
        print(f"\nOpen in browser: file://{Path(output_file).absolute()}")

    def create_clickbench_html(self) -> str:
        """Create ClickBench-style HTML report"""

        # Prepare data
        queries = sorted(self.results['queries'].keys())
        overall_stats = self.results.get('overall', {})

        # Calculate max times for bar scaling
        java_means = [self.results['queries'][q]['java_fe']['mean'] for q in queries]
        rust_means = [self.results['queries'][q]['rust_fe']['mean'] for q in queries]
        max_time = max(java_means + rust_means) if java_means and rust_means else 1.0

        # Start HTML
        html = f'''<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{self.benchmark_name} Benchmark — Java FE vs Rust FE</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter&display=swap" rel="stylesheet">

    <style>
        :root {{
            --color: black;
            --title-color: black;
            --background-color: white;
            --link-color: #08F;
            --link-hover-color: #F40;
            --summary-every-other-row-color: #F8F8F8;
            --highlight-color: #EEE;

            --bar-java-color: #FFA500;
            --bar-rust-color: #4CAF50;
            --bar-speedup-good: #4CAF50;
            --bar-speedup-bad: #FF5252;
        }}

        [data-theme="dark"] {{
            --color: #CCC;
            --title-color: white;
            --background-color: #04293A;
            --link-color: #8CF;
            --link-hover-color: #FFF;
            --summary-every-other-row-color: #042e41;
            --highlight-color: #064663;
        }}

        html, body {{
            color: var(--color);
            background-color: var(--background-color);
            width: 100%;
            height: 100%;
            margin: 0;
            font-family: 'Inter', sans-serif;
            padding: 1% 3% 0 3%;
            box-sizing: border-box;
        }}

        h1 {{
            color: var(--title-color);
            margin: 1rem 0 0.5rem 0;
        }}

        h2 {{
            color: var(--title-color);
            margin: 2rem 0 1rem 0;
        }}

        a, a:visited {{
            text-decoration: none;
            color: var(--link-color);
        }}

        a:hover {{
            color: var(--link-hover-color);
        }}

        table {{
            border-spacing: 1px;
            width: 100%;
        }}

        th {{
            padding-bottom: 0.5rem;
            text-align: left;
        }}

        #summary tr:nth-child(odd) {{
            background: var(--summary-every-other-row-color);
        }}

        .summary-row:hover {{
            background: var(--highlight-color) !important;
            font-weight: bold;
        }}

        .summary-name {{
            white-space: nowrap;
            text-align: left;
            padding: 0.5rem 1rem 0.5rem 0.5rem;
            font-weight: bold;
        }}

        .summary-bar-cell {{
            width: 100%;
            padding: 0.2rem 0;
        }}

        .summary-bar-container {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}

        .summary-bar {{
            height: 1.2rem;
            display: inline-block;
            transition: all 0.2s;
        }}

        .summary-bar-java {{
            background: var(--bar-java-color);
        }}

        .summary-bar-rust {{
            background: var(--bar-rust-color);
        }}

        .summary-number {{
            font-family: monospace;
            text-align: right;
            white-space: nowrap;
            min-width: 4rem;
        }}

        .metadata-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 1rem;
            margin: 1rem 0 2rem 0;
            padding: 1rem;
            background: var(--summary-every-other-row-color);
            border-radius: 0.5rem;
        }}

        .metadata-item {{
            text-align: center;
        }}

        .metadata-label {{
            font-size: 0.9rem;
            opacity: 0.7;
        }}

        .metadata-value {{
            font-size: 1.5rem;
            font-weight: bold;
            margin-top: 0.25rem;
        }}

        .theme-toggle {{
            position: fixed;
            top: 1rem;
            right: 1rem;
            padding: 0.5rem 1rem;
            background: var(--highlight-color);
            border: none;
            border-radius: 0.5rem;
            cursor: pointer;
            font-family: 'Inter', sans-serif;
            color: var(--color);
        }}

        .theme-toggle:hover {{
            opacity: 0.8;
        }}

        .details-table {{
            margin-top: 2rem;
        }}

        .details-table td, .details-table th {{
            padding: 0.5rem;
        }}

        .details-table tr:hover {{
            background: var(--highlight-color);
        }}

        .speedup-good {{
            color: var(--bar-speedup-good);
            font-weight: bold;
        }}

        .speedup-bad {{
            color: var(--bar-speedup-bad);
            font-weight: bold;
        }}
    </style>
</head>
<body>
    <button class="theme-toggle" onclick="toggleTheme()">🌓 Toggle Theme</button>

    <h1>{self.benchmark_name} Benchmark Results</h1>
    <p>Java FE vs Rust FE Performance Comparison</p>

    <div class="metadata-grid">
        <div class="metadata-item">
            <div class="metadata-label">Benchmark</div>
            <div class="metadata-value">{self.benchmark_name}</div>
        </div>
        <div class="metadata-item">
            <div class="metadata-label">Scale Factor</div>
            <div class="metadata-value">SF{self.results['metadata']['scale_factor']}</div>
        </div>
        <div class="metadata-item">
            <div class="metadata-label">Rounds</div>
            <div class="metadata-value">{self.results['metadata']['rounds']}</div>
        </div>
        <div class="metadata-item">
            <div class="metadata-label">Java FE</div>
            <div class="metadata-value">{self.results['metadata']['java_fe']}</div>
        </div>
        <div class="metadata-item">
            <div class="metadata-label">Rust FE</div>
            <div class="metadata-value">{self.results['metadata']['rust_fe']}</div>
        </div>
        <div class="metadata-item">
            <div class="metadata-label">Geom Mean Speedup</div>
            <div class="metadata-value">{overall_stats.get('geometric_mean', 0):.2f}x</div>
        </div>
    </div>

    <h2>Query Execution Times (Lower is Better)</h2>
    <table id="summary">
        <thead>
            <tr>
                <th class="summary-name">Query</th>
                <th>Java FE</th>
                <th>Rust FE</th>
                <th class="summary-number">Speedup</th>
            </tr>
        </thead>
        <tbody>
'''

        # Add summary rows
        for query_name in queries:
            data = self.results['queries'][query_name]
            java_mean = data['java_fe']['mean']
            rust_mean = data['rust_fe']['mean']
            speedup = data['speedup']['mean']

            # Calculate bar widths (percentage of max time)
            java_width = (java_mean / max_time) * 100
            rust_width = (rust_mean / max_time) * 100

            speedup_class = 'speedup-good' if speedup > 1.0 else 'speedup-bad'

            html += f'''            <tr class="summary-row">
                <td class="summary-name">{query_name}</td>
                <td class="summary-bar-cell">
                    <div class="summary-bar-container">
                        <div class="summary-bar summary-bar-java" style="width: {java_width:.1f}%;"></div>
                        <span class="summary-number">{java_mean:.3f}s</span>
                    </div>
                </td>
                <td class="summary-bar-cell">
                    <div class="summary-bar-container">
                        <div class="summary-bar summary-bar-rust" style="width: {rust_width:.1f}%;"></div>
                        <span class="summary-number">{rust_mean:.3f}s</span>
                    </div>
                </td>
                <td class="summary-number {speedup_class}">{speedup:.2f}x</td>
            </tr>
'''

        # Add detailed results table
        html += f'''        </tbody>
    </table>

    <h2>Detailed Results</h2>
    <table class="details-table">
        <thead>
            <tr>
                <th>Query</th>
                <th>Java FE Mean</th>
                <th>Java FE StdDev</th>
                <th>Rust FE Mean</th>
                <th>Rust FE StdDev</th>
                <th>Mean Speedup</th>
                <th>Median Speedup</th>
                <th>Best Speedup</th>
            </tr>
        </thead>
        <tbody>
'''

        for query_name in queries:
            data = self.results['queries'][query_name]
            speedup_class = 'speedup-good' if data['speedup']['mean'] > 1.0 else 'speedup-bad'

            html += f'''            <tr>
                <td><strong>{query_name}</strong></td>
                <td class="summary-number">{data['java_fe']['mean']:.3f}s</td>
                <td class="summary-number">±{data['java_fe']['stdev']:.3f}s</td>
                <td class="summary-number">{data['rust_fe']['mean']:.3f}s</td>
                <td class="summary-number">±{data['rust_fe']['stdev']:.3f}s</td>
                <td class="summary-number {speedup_class}">{data['speedup']['mean']:.2f}x</td>
                <td class="summary-number {speedup_class}">{data['speedup']['median']:.2f}x</td>
                <td class="summary-number {speedup_class}">{data['speedup']['best']:.2f}x</td>
            </tr>
'''

        # Close HTML
        html += f'''        </tbody>
    </table>

    <h2>Overall Statistics</h2>
    <table class="details-table">
        <tr>
            <td><strong>Geometric Mean Speedup</strong></td>
            <td class="summary-number speedup-good">{overall_stats.get('geometric_mean', 0):.2f}x</td>
        </tr>
        <tr>
            <td><strong>Arithmetic Mean Speedup</strong></td>
            <td class="summary-number speedup-good">{overall_stats.get('arithmetic_mean', 0):.2f}x</td>
        </tr>
        <tr>
            <td><strong>Median Speedup</strong></td>
            <td class="summary-number speedup-good">{overall_stats.get('median', 0):.2f}x</td>
        </tr>
        <tr>
            <td><strong>Best Speedup</strong></td>
            <td class="summary-number speedup-good">{overall_stats.get('best', 0):.2f}x</td>
        </tr>
        <tr>
            <td><strong>Worst Speedup</strong></td>
            <td class="summary-number">{overall_stats.get('worst', 0):.2f}x</td>
        </tr>
    </table>

    <p style="margin-top: 3rem; opacity: 0.6; text-align: center;">
        Generated on {datetime.now().strftime('%Y-%m-%d %H:%M:%S')} |
        {self.benchmark_name} Benchmark Suite |
        Inspired by <a href="https://benchmark.clickhouse.com/" target="_blank">ClickBench</a>
    </p>

    <script>
        function toggleTheme() {{
            const html = document.documentElement;
            const currentTheme = html.getAttribute('data-theme');
            const newTheme = currentTheme === 'dark' ? '' : 'dark';
            html.setAttribute('data-theme', newTheme);
            localStorage.setItem('theme', newTheme);
        }}

        // Load saved theme
        const savedTheme = localStorage.getItem('theme');
        if (savedTheme) {{
            document.documentElement.setAttribute('data-theme', savedTheme);
        }}
    </script>
</body>
</html>
'''

        return html


def main():
    parser = argparse.ArgumentParser(
        description='TPC-H/TPC-DS Benchmark Runner with ClickBench-style Visualization',
        formatter_class=argparse.RawDescriptionHelpFormatter
    )

    parser.add_argument('-s', '--scale', type=int, default=1,
                        help='Scale factor (default: 1)')
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
                        help='Directory containing query SQL files (default: scripts/tpch/queries)')

    parser.add_argument('--output-json', default='tpch_results.json',
                        help='Output JSON file (default: tpch_results.json)')
    parser.add_argument('--output-html', default='tpch_results.html',
                        help='Output HTML file (default: tpch_results.html)')

    parser.add_argument('--benchmark-name', default='TPC-H',
                        help='Benchmark name for display (default: TPC-H)')

    args = parser.parse_args()

    # Run benchmark
    runner = BenchmarkRunner(args)
    runner.run_benchmark()


if __name__ == '__main__':
    main()
