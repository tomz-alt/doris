#!/usr/bin/env python3
"""
TPC-H Benchmark Runner
Inspired by ClickHouse JSONBench approach
Runs TPC-H queries against Java FE and Rust FE, generates HTML report
"""

import argparse
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Tuple
import subprocess
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
        self.results = {
            'metadata': {
                'benchmark': 'TPC-H',
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
            conn = mysql.connector.connect(
                host=host,
                port=port,
                user='root',
                password='',
                database=f'tpch_sf{self.args.scale}',
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
        print(f"TPC-H Benchmark (SF{self.args.scale})")
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
        """Generate HTML report"""
        output_file = self.args.output_html

        html_content = self.create_html_report()

        with open(output_file, 'w') as f:
            f.write(html_content)

        print(f"HTML report saved to: {output_file}")
        print(f"\nOpen in browser: file://{Path(output_file).absolute()}")

    def create_html_report(self) -> str:
        """Create HTML report with charts"""

        # Prepare data for charts
        queries = sorted(self.results['queries'].keys())
        java_means = [self.results['queries'][q]['java_fe']['mean'] for q in queries]
        rust_means = [self.results['queries'][q]['rust_fe']['mean'] for q in queries]
        speedups = [self.results['queries'][q]['speedup']['mean'] for q in queries]

        # Create detailed query data
        query_details = []
        for q in queries:
            data = self.results['queries'][q]
            query_details.append({
                'name': q,
                'java_times': data['java_fe']['times'],
                'rust_times': data['rust_fe']['times'],
                'java_mean': data['java_fe']['mean'],
                'rust_mean': data['rust_fe']['mean'],
                'java_stdev': data['java_fe']['stdev'],
                'rust_stdev': data['rust_fe']['stdev'],
                'speedup': data['speedup']['mean']
            })

        overall_stats = self.results.get('overall', {})

        html = f'''<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>TPC-H Benchmark: Java FE vs Rust FE</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: #f5f7fa;
            color: #2c3e50;
            line-height: 1.6;
        }}

        .container {{
            max-width: 1400px;
            margin: 0 auto;
            padding: 20px;
        }}

        header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 40px 20px;
            margin: -20px -20px 40px -20px;
            border-radius: 0 0 20px 20px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        }}

        h1 {{
            font-size: 2.5em;
            margin-bottom: 10px;
            font-weight: 700;
        }}

        .subtitle {{
            font-size: 1.2em;
            opacity: 0.9;
        }}

        .metadata {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 20px;
            margin-bottom: 40px;
        }}

        .metadata-card {{
            background: white;
            padding: 20px;
            border-radius: 10px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}

        .metadata-card h3 {{
            color: #667eea;
            font-size: 0.9em;
            text-transform: uppercase;
            margin-bottom: 10px;
            font-weight: 600;
        }}

        .metadata-card p {{
            font-size: 1.5em;
            font-weight: 700;
            color: #2c3e50;
        }}

        .summary {{
            background: white;
            padding: 30px;
            border-radius: 10px;
            margin-bottom: 40px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}

        .summary h2 {{
            color: #2c3e50;
            margin-bottom: 20px;
            font-size: 1.8em;
        }}

        .summary-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-top: 20px;
        }}

        .summary-item {{
            text-align: center;
            padding: 20px;
            background: #f8f9fa;
            border-radius: 8px;
        }}

        .summary-item .label {{
            font-size: 0.9em;
            color: #6c757d;
            margin-bottom: 10px;
        }}

        .summary-item .value {{
            font-size: 2em;
            font-weight: 700;
            color: #667eea;
        }}

        .chart-container {{
            background: white;
            padding: 30px;
            border-radius: 10px;
            margin-bottom: 40px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}

        .chart-container h2 {{
            margin-bottom: 20px;
            color: #2c3e50;
            font-size: 1.5em;
        }}

        .chart-wrapper {{
            position: relative;
            height: 400px;
            margin-top: 20px;
        }}

        .results-table {{
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 40px;
            overflow-x: auto;
        }}

        .results-table h2 {{
            margin-bottom: 20px;
            color: #2c3e50;
            font-size: 1.5em;
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
        }}

        th, td {{
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #e9ecef;
        }}

        th {{
            background: #f8f9fa;
            font-weight: 600;
            color: #495057;
            position: sticky;
            top: 0;
        }}

        tr:hover {{
            background: #f8f9fa;
        }}

        .speedup-good {{
            color: #28a745;
            font-weight: 600;
        }}

        .speedup-bad {{
            color: #dc3545;
            font-weight: 600;
        }}

        .query-detail {{
            background: white;
            padding: 25px;
            border-radius: 10px;
            margin-bottom: 30px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}

        .query-detail h3 {{
            color: #667eea;
            margin-bottom: 15px;
            font-size: 1.3em;
        }}

        .rounds-display {{
            display: flex;
            gap: 10px;
            flex-wrap: wrap;
            margin-top: 15px;
        }}

        .round-badge {{
            padding: 8px 12px;
            background: #e9ecef;
            border-radius: 5px;
            font-size: 0.9em;
        }}

        .round-badge.java {{
            background: #ffeaa7;
        }}

        .round-badge.rust {{
            background: #74b9ff;
        }}

        footer {{
            text-align: center;
            padding: 20px;
            color: #6c757d;
            font-size: 0.9em;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>📊 TPC-H Benchmark Results</h1>
            <div class="subtitle">Java FE vs Rust FE Performance Comparison</div>
        </header>

        <div class="metadata">
            <div class="metadata-card">
                <h3>Scale Factor</h3>
                <p>SF{self.results['metadata']['scale_factor']}</p>
            </div>
            <div class="metadata-card">
                <h3>Benchmark Rounds</h3>
                <p>{self.results['metadata']['rounds']}</p>
            </div>
            <div class="metadata-card">
                <h3>Java FE</h3>
                <p>{self.results['metadata']['java_fe']}</p>
            </div>
            <div class="metadata-card">
                <h3>Rust FE</h3>
                <p>{self.results['metadata']['rust_fe']}</p>
            </div>
            <div class="metadata-card">
                <h3>Timestamp</h3>
                <p>{self.results['metadata']['timestamp'][:10]}</p>
            </div>
        </div>

        <div class="summary">
            <h2>Overall Performance</h2>
            <div class="summary-grid">
                <div class="summary-item">
                    <div class="label">Geometric Mean Speedup</div>
                    <div class="value">{overall_stats.get('geometric_mean', 0):.2f}x</div>
                </div>
                <div class="summary-item">
                    <div class="label">Average Speedup</div>
                    <div class="value">{overall_stats.get('arithmetic_mean', 0):.2f}x</div>
                </div>
                <div class="summary-item">
                    <div class="label">Median Speedup</div>
                    <div class="value">{overall_stats.get('median', 0):.2f}x</div>
                </div>
                <div class="summary-item">
                    <div class="label">Best Speedup</div>
                    <div class="value">{overall_stats.get('best', 0):.2f}x</div>
                </div>
                <div class="summary-item">
                    <div class="label">Worst Speedup</div>
                    <div class="value">{overall_stats.get('worst', 0):.2f}x</div>
                </div>
            </div>
        </div>

        <div class="chart-container">
            <h2>Query Execution Time Comparison</h2>
            <div class="chart-wrapper">
                <canvas id="timeComparisonChart"></canvas>
            </div>
        </div>

        <div class="chart-container">
            <h2>Speedup Factor by Query</h2>
            <div class="chart-wrapper">
                <canvas id="speedupChart"></canvas>
            </div>
        </div>

        <div class="results-table">
            <h2>Detailed Results</h2>
            <table>
                <thead>
                    <tr>
                        <th>Query</th>
                        <th>Java FE Mean (s)</th>
                        <th>Rust FE Mean (s)</th>
                        <th>Speedup</th>
                        <th>Java FE StdDev</th>
                        <th>Rust FE StdDev</th>
                    </tr>
                </thead>
                <tbody>
'''

        # Add table rows
        for detail in query_details:
            speedup_class = 'speedup-good' if detail['speedup'] > 1.0 else 'speedup-bad'
            html += f'''
                    <tr>
                        <td><strong>{detail['name']}</strong></td>
                        <td>{detail['java_mean']:.3f}</td>
                        <td>{detail['rust_mean']:.3f}</td>
                        <td class="{speedup_class}">{detail['speedup']:.2f}x</td>
                        <td>±{detail['java_stdev']:.3f}</td>
                        <td>±{detail['rust_stdev']:.3f}</td>
                    </tr>
'''

        html += '''
                </tbody>
            </table>
        </div>

        <h2 style="margin-bottom: 20px; color: #2c3e50;">Query-by-Query Analysis</h2>
'''

        # Add individual query details
        for detail in query_details:
            html += f'''
        <div class="query-detail">
            <h3>{detail['name']}</h3>
            <p><strong>Mean Speedup:</strong> <span class="{'speedup-good' if detail['speedup'] > 1.0 else 'speedup-bad'}">{detail['speedup']:.2f}x</span></p>
            <p><strong>Java FE:</strong> {detail['java_mean']:.3f}s ± {detail['java_stdev']:.3f}s</p>
            <p><strong>Rust FE:</strong> {detail['rust_mean']:.3f}s ± {detail['rust_stdev']:.3f}s</p>

            <div class="rounds-display">
'''

            for i, (java_time, rust_time) in enumerate(zip(detail['java_times'], detail['rust_times']), 1):
                html += f'''
                <div class="round-badge java">Round {i} Java: {java_time:.3f}s</div>
                <div class="round-badge rust">Round {i} Rust: {rust_time:.3f}s</div>
'''

            html += '''
            </div>
        </div>
'''

        # Close HTML and add JavaScript
        html += f'''
        <footer>
            <p>Generated on {datetime.now().strftime('%Y-%m-%d %H:%M:%S')} | TPC-H Benchmark Suite</p>
        </footer>
    </div>

    <script>
        // Time comparison chart
        const timeCtx = document.getElementById('timeComparisonChart').getContext('2d');
        new Chart(timeCtx, {{
            type: 'bar',
            data: {{
                labels: {json.dumps(queries)},
                datasets: [
                    {{
                        label: 'Java FE',
                        data: {json.dumps(java_means)},
                        backgroundColor: 'rgba(255, 206, 86, 0.6)',
                        borderColor: 'rgba(255, 206, 86, 1)',
                        borderWidth: 2
                    }},
                    {{
                        label: 'Rust FE',
                        data: {json.dumps(rust_means)},
                        backgroundColor: 'rgba(54, 162, 235, 0.6)',
                        borderColor: 'rgba(54, 162, 235, 1)',
                        borderWidth: 2
                    }}
                ]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                plugins: {{
                    title: {{
                        display: true,
                        text: 'Lower is Better',
                        font: {{ size: 14 }}
                    }},
                    legend: {{
                        position: 'top',
                    }}
                }},
                scales: {{
                    y: {{
                        beginAtZero: true,
                        title: {{
                            display: true,
                            text: 'Execution Time (seconds)'
                        }}
                    }}
                }}
            }}
        }});

        // Speedup chart
        const speedupCtx = document.getElementById('speedupChart').getContext('2d');
        new Chart(speedupCtx, {{
            type: 'bar',
            data: {{
                labels: {json.dumps(queries)},
                datasets: [
                    {{
                        label: 'Speedup Factor',
                        data: {json.dumps(speedups)},
                        backgroundColor: {json.dumps([
                            'rgba(75, 192, 192, 0.6)' if s > 1.0 else 'rgba(255, 99, 132, 0.6)'
                            for s in speedups
                        ])},
                        borderColor: {json.dumps([
                            'rgba(75, 192, 192, 1)' if s > 1.0 else 'rgba(255, 99, 132, 1)'
                            for s in speedups
                        ])},
                        borderWidth: 2
                    }}
                ]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                plugins: {{
                    title: {{
                        display: true,
                        text: 'Higher is Better (>1.0 means Rust FE is faster)',
                        font: {{ size: 14 }}
                    }},
                    legend: {{
                        display: false
                    }}
                }},
                scales: {{
                    y: {{
                        beginAtZero: true,
                        title: {{
                            display: true,
                            text: 'Speedup Factor (Java FE / Rust FE)'
                        }}
                    }}
                }}
            }}
        }});
    </script>
</body>
</html>
'''

        return html


def main():
    parser = argparse.ArgumentParser(
        description='TPC-H Benchmark Runner',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
Examples:
  # Run SF1 benchmark with 3 rounds
  python3 benchmark_tpch.py --scale 1 --rounds 3

  # Run SF10 with 5 rounds and 2 warmup rounds
  python3 benchmark_tpch.py --scale 10 --rounds 5 --warmup 2

  # Use custom FE ports
  python3 benchmark_tpch.py --java-port 9030 --rust-port 9031
        '''
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
                        help='Directory containing query SQL files (default: scripts/tpch/queries)')

    parser.add_argument('--output-json', default='tpch_results.json',
                        help='Output JSON file (default: tpch_results.json)')
    parser.add_argument('--output-html', default='tpch_results.html',
                        help='Output HTML file (default: tpch_results.html)')

    args = parser.parse_args()

    # Run benchmark
    runner = BenchmarkRunner(args)
    runner.run_benchmark()


if __name__ == '__main__':
    main()
