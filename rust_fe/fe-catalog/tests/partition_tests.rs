// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Tests for Partition module - verify behavior parity with Java FE Partition.java

use fe_catalog::Partition;
use fe_common::StorageMedium;

#[test]
fn test_partition_creation() {
    let partition = Partition::new(1, "p1".to_string(), 3);

    assert_eq!(partition.id, 1);
    assert_eq!(partition.name, "p1");
    assert_eq!(partition.replication_num, 3);
    assert_eq!(partition.visible_version, 1);
    assert_eq!(partition.next_version, 2);
    assert_eq!(partition.data_size, 0);
    assert_eq!(partition.row_count, 0);
    assert_eq!(partition.storage_medium, StorageMedium::Hdd);
    assert!(!partition.is_temp);
}

#[test]
fn test_partition_tablets() {
    let partition = Partition::new(1, "p1".to_string(), 3);

    // Add tablets
    partition.add_tablet(1001, 1);
    partition.add_tablet(1002, 1);
    partition.add_tablet(1003, 1);

    assert_eq!(partition.tablet_count(), 3);
}

#[test]
fn test_partition_version_update() {
    let mut partition = Partition::new(1, "p1".to_string(), 3);

    assert_eq!(partition.visible_version, 1);
    assert_eq!(partition.next_version, 2);

    partition.update_visible_version(5);

    assert_eq!(partition.visible_version, 5);
    assert_eq!(partition.next_version, 6);
}

#[test]
fn test_partition_storage_medium() {
    let mut partition = Partition::new(1, "p1".to_string(), 3);

    partition.storage_medium = StorageMedium::Ssd;
    assert_eq!(partition.storage_medium, StorageMedium::Ssd);

    partition.storage_medium = StorageMedium::S3;
    assert_eq!(partition.storage_medium, StorageMedium::S3);
}

#[test]
fn test_temporary_partition() {
    let mut partition = Partition::new(1, "temp_p1".to_string(), 3);
    partition.is_temp = true;

    assert!(partition.is_temp);
}
