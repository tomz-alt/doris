// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Replica definition

use fe_common::{ReplicaId, BackendId, TabletId};
use fe_common::types::ReplicaState;
use serde::{Deserialize, Serialize};

/// Replica metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replica {
    /// Replica ID
    pub id: ReplicaId,

    /// Backend ID where this replica resides
    pub backend_id: BackendId,

    /// Tablet ID
    pub tablet_id: TabletId,

    /// Version
    pub version: i64,

    /// Last failed version
    pub last_failed_version: i64,

    /// Last success version
    pub last_success_version: i64,

    /// Data size (bytes)
    pub data_size: i64,

    /// Row count
    pub row_count: i64,

    /// Replica state
    pub state: ReplicaState,

    /// Last update time (milliseconds)
    pub last_update_time: u64,

    /// Path hash
    pub path_hash: i64,

    /// Is bad replica
    pub is_bad: bool,
}

impl Replica {
    pub fn new(id: ReplicaId, backend_id: BackendId, tablet_id: TabletId) -> Self {
        Self {
            id,
            backend_id,
            tablet_id,
            version: 1,
            last_failed_version: -1,
            last_success_version: 1,
            data_size: 0,
            row_count: 0,
            state: ReplicaState::Normal,
            last_update_time: fe_common::utils::current_timestamp_ms(),
            path_hash: 0,
            is_bad: false,
        }
    }

    /// Update version
    pub fn update_version(&mut self, version: i64, data_size: i64, row_count: i64) {
        self.version = version;
        self.last_success_version = version;
        self.data_size = data_size;
        self.row_count = row_count;
        self.last_update_time = fe_common::utils::current_timestamp_ms();
    }

    /// Mark as bad replica
    pub fn mark_bad(&mut self) {
        self.is_bad = true;
        self.state = ReplicaState::Bad;
        self.last_update_time = fe_common::utils::current_timestamp_ms();
    }

    /// Check if replica is healthy
    pub fn is_healthy(&self) -> bool {
        !self.is_bad && self.state == ReplicaState::Normal
    }
}
