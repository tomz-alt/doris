/// Integration test to validate feature flags work correctly
use doris_rust_fe::planner::feature_flags::QueryFeatureFlags;
use std::env;
use std::fs;

#[test]
fn test_default_flags() {
    let flags = QueryFeatureFlags::default();
    assert_eq!(flags.cost_based_join_strategy, false);
    assert_eq!(flags.advanced_partition_pruning, false);
    assert_eq!(flags.arrow_result_parsing, false);
    assert_eq!(flags.runtime_filter_propagation, false);
    assert_eq!(flags.bucket_shuffle_optimization, false);
    assert_eq!(flags.parallel_fragment_execution, true);
    println!("✓ Default flags validated");
}

#[test]
fn test_all_execution_preset() {
    let flags = QueryFeatureFlags::preset("all-execution");
    assert_eq!(flags.cost_based_join_strategy, true);
    assert_eq!(flags.advanced_partition_pruning, true);
    assert_eq!(flags.arrow_result_parsing, false); // Not included in all-execution
    assert_eq!(flags.runtime_filter_propagation, true);
    assert_eq!(flags.bucket_shuffle_optimization, true);
    println!("✓ All execution optimizations preset validated");
}

#[test]
fn test_join_optimization_preset() {
    let flags = QueryFeatureFlags::preset("join-optimization");
    assert_eq!(flags.cost_based_join_strategy, true);
    assert_eq!(flags.advanced_partition_pruning, false);
    assert_eq!(flags.runtime_filter_propagation, false);
    println!("✓ Join optimization preset validated");
}

#[test]
fn test_partition_pruning_preset() {
    let flags = QueryFeatureFlags::preset("partition-pruning");
    assert_eq!(flags.cost_based_join_strategy, false);
    assert_eq!(flags.advanced_partition_pruning, true);
    println!("✓ Partition pruning preset validated");
}

#[test]
fn test_runtime_filters_preset() {
    let flags = QueryFeatureFlags::preset("runtime-filters");
    assert_eq!(flags.runtime_filter_propagation, true);
    println!("✓ Runtime filters preset validated");
}

#[test]
fn test_toml_config() {
    let config = r#"
cost_based_join_strategy = true
advanced_partition_pruning = true
broadcast_threshold_bytes = 52428800
max_fragment_parallelism = 16
"#;

    let temp_file = "/tmp/test_feature_flags.toml";
    fs::write(temp_file, config).unwrap();

    let flags = QueryFeatureFlags::from_toml(temp_file).unwrap();
    assert_eq!(flags.cost_based_join_strategy, true);
    assert_eq!(flags.advanced_partition_pruning, true);
    assert_eq!(flags.broadcast_threshold_bytes, 52428800);
    assert_eq!(flags.max_fragment_parallelism, 16);

    fs::remove_file(temp_file).ok();
    println!("✓ TOML config loading validated");
}

#[test]
fn test_env_var_override() {
    env::set_var("DORIS_COST_BASED_JOIN", "true");
    env::set_var("DORIS_PARTITION_PRUNING", "true");
    env::set_var("DORIS_BROADCAST_THRESHOLD", "52428800");

    let flags = QueryFeatureFlags::from_env();
    assert_eq!(flags.cost_based_join_strategy, true);
    assert_eq!(flags.advanced_partition_pruning, true);
    assert_eq!(flags.broadcast_threshold_bytes, 52428800);

    env::remove_var("DORIS_COST_BASED_JOIN");
    env::remove_var("DORIS_PARTITION_PRUNING");
    env::remove_var("DORIS_BROADCAST_THRESHOLD");
    println!("✓ Environment variable override validated");
}

#[test]
fn test_all_presets() {
    let presets = vec![
        "baseline",
        "join-optimization",
        "partition-pruning",
        "arrow-format",
        "runtime-filters",
        "bucket-shuffle",
        "all-execution",
    ];

    for preset in presets {
        let flags = QueryFeatureFlags::preset(preset);
        println!("✓ Preset '{}' loaded successfully", preset);

        // Verify parallel execution is always true
        assert_eq!(flags.parallel_fragment_execution, true,
            "Preset '{}' should have parallel execution enabled", preset);
    }
}

#[test]
fn test_feature_flag_combinations() {
    // Test baseline + cost-based join
    let mut flags = QueryFeatureFlags::default();
    flags.cost_based_join_strategy = true;
    assert!(flags.cost_based_join_strategy);
    assert!(!flags.advanced_partition_pruning);
    println!("✓ Baseline + cost-based join validated");

    // Test baseline + partition pruning
    let mut flags = QueryFeatureFlags::default();
    flags.advanced_partition_pruning = true;
    assert!(!flags.cost_based_join_strategy);
    assert!(flags.advanced_partition_pruning);
    println!("✓ Baseline + partition pruning validated");

    // Test all enabled
    let mut flags = QueryFeatureFlags::default();
    flags.cost_based_join_strategy = true;
    flags.advanced_partition_pruning = true;
    flags.arrow_result_parsing = true;
    flags.runtime_filter_propagation = true;
    flags.bucket_shuffle_optimization = true;
    assert!(flags.cost_based_join_strategy);
    assert!(flags.advanced_partition_pruning);
    assert!(flags.arrow_result_parsing);
    assert!(flags.runtime_filter_propagation);
    assert!(flags.bucket_shuffle_optimization);
    println!("✓ All optimizations enabled validated");
}
