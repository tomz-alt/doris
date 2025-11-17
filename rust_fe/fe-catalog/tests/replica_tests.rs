// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Tests for Replica module - verify behavior parity with Java FE Replica.java

use fe_catalog::Replica;
use fe_common::ReplicaState;

#[test]
fn test_replica_creation() {
    let replica = Replica::new(1, 100, 1001);

    assert_eq!(replica.id, 1);
    assert_eq!(replica.backend_id, 100);
    assert_eq!(replica.tablet_id, 1001);
    assert_eq!(replica.version, 1);
    assert_eq!(replica.last_success_version, 1);
    assert_eq!(replica.last_failed_version, -1);
    assert_eq!(replica.state, ReplicaState::Normal);
    assert!(!replica.is_bad);
}

#[test]
fn test_replica_update_version() {
    let mut replica = Replica::new(1, 100, 1001);

    replica.update_version(5, 1024 * 1024, 1000);

    assert_eq!(replica.version, 5);
    assert_eq!(replica.last_success_version, 5);
    assert_eq!(replica.data_size, 1024 * 1024);
    assert_eq!(replica.row_count, 1000);
}

#[test]
fn test_replica_mark_bad() {
    let mut replica = Replica::new(1, 100, 1001);

    assert!(replica.is_healthy());

    replica.mark_bad();

    assert!(replica.is_bad);
    assert_eq!(replica.state, ReplicaState::Bad);
    assert!(!replica.is_healthy());
}

#[test]
fn test_replica_states() {
    let mut replica = Replica::new(1, 100, 1001);

    // Test all states
    replica.state = ReplicaState::Normal;
    assert!(replica.is_healthy());

    replica.state = ReplicaState::Clone;
    assert!(!replica.is_healthy());

    replica.state = ReplicaState::SchemaChange;
    assert!(!replica.is_healthy());

    replica.state = ReplicaState::Bad;
    assert!(!replica.is_healthy());
}
