//! TPC-H Benchmark Suite for Distributed Query Optimizations
//!
//! Validates optimization effectiveness by benchmarking:
//! 1. SQL parsing with Doris-specific syntax
//! 2. Cost model decision-making
//! 3. Feature flag configuration
//!
//! Note: This benchmark focuses on synchronous FE operations.
//! For end-to-end performance, use the E2E comparison script.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use doris_rust_fe::planner::feature_flags::QueryFeatureFlags;
use doris_rust_fe::planner::cost_model::{JoinCostModel, TableStatistics};
use doris_rust_fe::parser::doris_parser::DorisParser;
use std::time::Duration;

/// TPC-H benchmark queries (simplified versions)
const TPCH_QUERIES: &[(&str, &str)] = &[
    // Q1: Pricing Summary Report (uses aggregation + partition pruning)
    ("Q1", "SELECT
        l_returnflag,
        l_linestatus,
        SUM(l_quantity) as sum_qty,
        SUM(l_extendedprice) as sum_base_price,
        SUM(l_extendedprice * (1 - l_discount)) as sum_disc_price,
        COUNT(*) as count_order
    FROM lineitem
    WHERE l_shipdate <= '1998-12-01'
    GROUP BY l_returnflag, l_linestatus
    ORDER BY l_returnflag, l_linestatus"),

    // Q3: Shipping Priority (uses joins + partition pruning)
    ("Q3", "SELECT
        l_orderkey,
        SUM(l_extendedprice * (1 - l_discount)) as revenue,
        o_orderdate,
        o_shippriority
    FROM customer, orders, lineitem
    WHERE c_mktsegment = 'BUILDING'
        AND c_custkey = o_custkey
        AND l_orderkey = o_orderkey
        AND o_orderdate < '1995-03-15'
        AND l_shipdate > '1995-03-15'
    GROUP BY l_orderkey, o_orderdate, o_shippriority
    ORDER BY revenue DESC, o_orderdate
    LIMIT 10"),

    // Q6: Forecasting Revenue Change (uses partition pruning)
    ("Q6", "SELECT
        SUM(l_extendedprice * l_discount) as revenue
    FROM lineitem
    WHERE l_shipdate >= '1994-01-01'
        AND l_shipdate < '1995-01-01'
        AND l_discount BETWEEN 0.05 AND 0.07
        AND l_quantity < 24"),

    // Q14: Promotion Effect (uses joins + aggregation)
    ("Q14", "SELECT
        100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%'
            THEN l_extendedprice * (1 - l_discount)
            ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) as promo_revenue
    FROM lineitem, part
    WHERE l_partkey = p_partkey
        AND l_shipdate >= '1995-09-01'
        AND l_shipdate < '1995-10-01'"),
];

/// Doris-specific SQL queries
const DORIS_QUERIES: &[(&str, &str)] = &[
    ("CREATE_TABLE", "CREATE TABLE test (
        id INT,
        name VARCHAR(100),
        value DECIMAL(10, 2)
    )
    PARTITION BY RANGE(id) (
        PARTITION p1 VALUES LESS THAN (100),
        PARTITION p2 VALUES LESS THAN (200),
        PARTITION p3 VALUES LESS THAN (300)
    )
    DISTRIBUTED BY HASH(id) BUCKETS 10
    PROPERTIES ('replication_num' = '1')"),

    ("MTMV", "CREATE MATERIALIZED VIEW test_mv
        BUILD IMMEDIATE REFRESH COMPLETE ON MANUAL
        AS SELECT id, COUNT(*) FROM test GROUP BY id"),

    ("ALTER_PARTITION", "ALTER TABLE test
        ADD PARTITION p4 VALUES LESS THAN (400)
        DISTRIBUTED BY HASH(id) BUCKETS 10"),

    ("INDEX", "CREATE INDEX idx_name ON test(name) USING INVERTED"),
];

/// Benchmark SQL parsing performance
fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sql_parsing");

    // Benchmark TPC-H queries
    for (query_name, query_sql) in TPCH_QUERIES.iter() {
        group.bench_with_input(
            BenchmarkId::new("tpch_query", query_name),
            query_sql,
            |b, sql| {
                b.iter(|| {
                    DorisParser::parse_sql(black_box(sql))
                });
            },
        );
    }

    // Benchmark Doris-specific queries
    for (query_name, query_sql) in DORIS_QUERIES.iter() {
        group.bench_with_input(
            BenchmarkId::new("doris_specific", query_name),
            query_sql,
            |b, sql| {
                b.iter(|| {
                    DorisParser::parse_sql(black_box(sql))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cost model performance
fn bench_cost_model(c: &mut Criterion) {
    let mut group = c.benchmark_group("cost_model");

    let cost_model = JoinCostModel::new(10);

    // Test scenarios
    let scenarios = vec![
        ("small_small",
            TableStatistics::estimated(1_000, 100),
            TableStatistics::estimated(2_000, 100)),
        ("small_large",
            TableStatistics::estimated(1_000, 100),
            TableStatistics::estimated(10_000_000, 200)),
        ("large_large",
            TableStatistics::estimated(10_000_000, 200),
            TableStatistics::estimated(15_000_000, 200)),
    ];

    for (scenario_name, left_table, right_table) in scenarios {
        group.bench_with_input(
            BenchmarkId::new("join_strategy_decision", scenario_name),
            &(left_table, right_table),
            |b, (left, right)| {
                b.iter(|| {
                    cost_model.choose_join_strategy(
                        black_box(left),
                        black_box(right),
                        black_box(10 * 1024 * 1024), // 10MB threshold
                    )
                });
            },
        );
    }

    // Benchmark cost estimation functions
    let left_stats = TableStatistics::estimated(10_000_000, 200);
    let right_stats = TableStatistics::estimated(1_000, 100);

    group.bench_function("estimate_broadcast_join", |b| {
        b.iter(|| {
            cost_model.estimate_broadcast_join(
                black_box(&right_stats),
                black_box(&left_stats),
            )
        });
    });

    group.bench_function("estimate_shuffle_join", |b| {
        b.iter(|| {
            cost_model.estimate_shuffle_join(
                black_box(&left_stats),
                black_box(&right_stats),
            )
        });
    });

    group.finish();
}

/// Benchmark feature flag operations
fn bench_feature_flags(c: &mut Criterion) {
    let mut group = c.benchmark_group("feature_flags");

    // Benchmark preset loading
    let presets = vec![
        "baseline",
        "join-optimization",
        "partition-pruning",
        "runtime-filters",
        "all-execution",
    ];

    for preset_name in presets {
        group.bench_with_input(
            BenchmarkId::new("preset_load", preset_name),
            &preset_name,
            |b, preset| {
                b.iter(|| {
                    QueryFeatureFlags::preset(black_box(preset))
                });
            },
        );
    }

    // Benchmark default creation
    group.bench_function("default_creation", |b| {
        b.iter(|| {
            QueryFeatureFlags::default()
        });
    });

    // Benchmark all_enabled creation
    group.bench_function("all_enabled_creation", |b| {
        b.iter(|| {
            QueryFeatureFlags::all_enabled()
        });
    });

    group.finish();
}

/// Benchmark table statistics operations
fn bench_table_statistics(c: &mut Criterion) {
    let mut group = c.benchmark_group("table_statistics");

    group.bench_function("estimated_creation", |b| {
        b.iter(|| {
            TableStatistics::estimated(
                black_box(1_000_000),
                black_box(150),
            )
        });
    });

    group.bench_function("is_broadcast_eligible_small", |b| {
        let small_stats = TableStatistics::estimated(1_000, 100);
        b.iter(|| {
            small_stats.is_broadcast_eligible(black_box(10 * 1024 * 1024))
        });
    });

    group.bench_function("is_broadcast_eligible_large", |b| {
        let large_stats = TableStatistics::estimated(10_000_000, 200);
        b.iter(|| {
            large_stats.is_broadcast_eligible(black_box(10 * 1024 * 1024))
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);
    targets =
        bench_parsing,
        bench_cost_model,
        bench_feature_flags,
        bench_table_statistics
}

criterion_main!(benches);
