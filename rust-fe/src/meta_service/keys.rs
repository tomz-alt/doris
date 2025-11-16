// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Key encoding for Doris cloud metadata storage
//!
//! This module implements the hierarchical key encoding scheme used in Doris
//! metadata storage, matching the C++ implementation in cloud/src/meta-store/keys.h
//!
//! # Key Spaces
//!
//! - `0x01` - User key space (instances, txns, metadata, jobs, etc.)
//! - `0x02` - System key space (registry, encryption keys)
//! - `0x03` - Versioned key space (snapshots, operation logs)

use bytes::{BufMut, BytesMut};

/// Key space prefixes
pub const CLOUD_USER_KEY_SPACE: u8 = 0x01;
pub const CLOUD_SYS_KEY_SPACE: u8 = 0x02;
pub const CLOUD_VERSIONED_KEY_SPACE: u8 = 0x03;

/// Separator for key components
const KEY_SEP: &[u8] = b"\x00";

/// Stats key suffixes
pub const STATS_KEY_SUFFIX_DATA_SIZE: &str = "data_size";
pub const STATS_KEY_SUFFIX_NUM_ROWS: &str = "num_rows";
pub const STATS_KEY_SUFFIX_NUM_ROWSETS: &str = "num_rowsets";
pub const STATS_KEY_SUFFIX_NUM_SEGS: &str = "num_segs";
pub const STATS_KEY_SUFFIX_INDEX_SIZE: &str = "index_size";
pub const STATS_KEY_SUFFIX_SEGMENT_SIZE: &str = "segment_size";

/// Helper to encode a byte slice into the buffer
#[inline]
fn encode_bytes(buf: &mut BytesMut, data: &[u8]) {
    buf.put_slice(data);
    buf.put_slice(KEY_SEP);
}

/// Helper to encode a string into the buffer
#[inline]
fn encode_str(buf: &mut BytesMut, s: &str) {
    encode_bytes(buf, s.as_bytes());
}

/// Helper to encode an i64 into the buffer (big-endian for sorting)
#[inline]
fn encode_i64(buf: &mut BytesMut, val: i64) {
    buf.put_i64(val);
    buf.put_slice(KEY_SEP);
}

/// Helper to encode a u64 into the buffer (big-endian for sorting)
#[inline]
fn encode_u64(buf: &mut BytesMut, val: u64) {
    buf.put_u64(val);
    buf.put_slice(KEY_SEP);
}

//==============================================================================
// Instance Keys
//==============================================================================

/// 0x01 "instance" ${instance_id} -> InstanceInfoPB
pub fn instance_key(instance_id: &str) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "instance");
    encode_str(&mut buf, instance_id);
    buf.to_vec()
}

//==============================================================================
// Transaction Keys
//==============================================================================

/// 0x01 "txn" ${instance_id} "txn_label" ${db_id} ${label} -> TxnLabelPB
pub fn txn_label_key(instance_id: &str, db_id: i64, label: &str) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "txn");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "txn_label");
    encode_i64(&mut buf, db_id);
    encode_str(&mut buf, label);
    buf.to_vec()
}

/// 0x01 "txn" ${instance_id} "txn_info" ${db_id} ${txn_id} -> TxnInfoPB
pub fn txn_info_key(instance_id: &str, db_id: i64, txn_id: i64) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "txn");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "txn_info");
    encode_i64(&mut buf, db_id);
    encode_i64(&mut buf, txn_id);
    buf.to_vec()
}

/// 0x01 "txn" ${instance_id} "txn_db_tbl" ${txn_id} -> TxnIndexPB
pub fn txn_index_key(instance_id: &str, txn_id: i64) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "txn");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "txn_db_tbl");
    encode_i64(&mut buf, txn_id);
    buf.to_vec()
}

/// 0x01 "txn" ${instance_id} "txn_running" ${db_id} ${txn_id} -> TxnRunningPB
pub fn txn_running_key(instance_id: &str, db_id: i64, txn_id: i64) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "txn");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "txn_running");
    encode_i64(&mut buf, db_id);
    encode_i64(&mut buf, txn_id);
    buf.to_vec()
}

//==============================================================================
// Version Keys
//==============================================================================

/// 0x01 "version" ${instance_id} "partition" ${db_id} ${tbl_id} ${partition_id} -> VersionPB
pub fn partition_version_key(
    instance_id: &str,
    db_id: i64,
    table_id: i64,
    partition_id: i64,
) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "version");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "partition");
    encode_i64(&mut buf, db_id);
    encode_i64(&mut buf, table_id);
    encode_i64(&mut buf, partition_id);
    buf.to_vec()
}

/// 0x01 "version" ${instance_id} "table" ${db_id} ${tbl_id} -> int64
pub fn table_version_key(instance_id: &str, db_id: i64, table_id: i64) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "version");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "table");
    encode_i64(&mut buf, db_id);
    encode_i64(&mut buf, table_id);
    buf.to_vec()
}

//==============================================================================
// Metadata Keys
//==============================================================================

/// 0x01 "meta" ${instance_id} "rowset" ${tablet_id} ${version} -> RowsetMetaCloudPB
pub fn rowset_meta_key(instance_id: &str, tablet_id: i64, version: i64) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "meta");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "rowset");
    encode_i64(&mut buf, tablet_id);
    encode_i64(&mut buf, version);
    buf.to_vec()
}

/// 0x01 "meta" ${instance_id} "rowset_tmp" ${txn_id} ${tablet_id} -> RowsetMetaCloudPB
pub fn rowset_tmp_key(instance_id: &str, txn_id: i64, tablet_id: i64) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "meta");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "rowset_tmp");
    encode_i64(&mut buf, txn_id);
    encode_i64(&mut buf, tablet_id);
    buf.to_vec()
}

/// 0x01 "meta" ${instance_id} "tablet" ${table_id} ${index_id} ${partition_id} ${tablet_id} -> TabletMetaCloudPB
pub fn tablet_meta_key(
    instance_id: &str,
    table_id: i64,
    index_id: i64,
    partition_id: i64,
    tablet_id: i64,
) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "meta");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "tablet");
    encode_i64(&mut buf, table_id);
    encode_i64(&mut buf, index_id);
    encode_i64(&mut buf, partition_id);
    encode_i64(&mut buf, tablet_id);
    buf.to_vec()
}

/// 0x01 "meta" ${instance_id} "tablet_index" ${tablet_id} -> TabletIndexPB
pub fn tablet_index_key(instance_id: &str, tablet_id: i64) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "meta");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "tablet_index");
    encode_i64(&mut buf, tablet_id);
    buf.to_vec()
}

/// 0x01 "meta" ${instance_id} "schema" ${index_id} ${schema_version} -> TabletSchemaCloudPB
pub fn schema_key(instance_id: &str, index_id: i64, schema_version: i64) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "meta");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "schema");
    encode_i64(&mut buf, index_id);
    encode_i64(&mut buf, schema_version);
    buf.to_vec()
}

//==============================================================================
// Tablet Stats Keys
//==============================================================================

/// 0x01 "stats" ${instance_id} "tablet" ${table_id} ${index_id} ${partition_id} ${tablet_id} -> TabletStatsPB
pub fn tablet_stats_key(
    instance_id: &str,
    table_id: i64,
    index_id: i64,
    partition_id: i64,
    tablet_id: i64,
) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "stats");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "tablet");
    encode_i64(&mut buf, table_id);
    encode_i64(&mut buf, index_id);
    encode_i64(&mut buf, partition_id);
    encode_i64(&mut buf, tablet_id);
    buf.to_vec()
}

/// 0x01 "stats" ${instance_id} "tablet" ... "data_size" -> int64
pub fn tablet_stats_data_size_key(
    instance_id: &str,
    table_id: i64,
    index_id: i64,
    partition_id: i64,
    tablet_id: i64,
) -> Vec<u8> {
    let mut key = tablet_stats_key(instance_id, table_id, index_id, partition_id, tablet_id);
    key.extend_from_slice(STATS_KEY_SUFFIX_DATA_SIZE.as_bytes());
    key.extend_from_slice(KEY_SEP);
    key
}

//==============================================================================
// Job Keys
//==============================================================================

/// 0x01 "job" ${instance_id} "tablet" ${table_id} ${index_id} ${partition_id} ${tablet_id} -> TabletJobInfoPB
pub fn tablet_job_key(
    instance_id: &str,
    table_id: i64,
    index_id: i64,
    partition_id: i64,
    tablet_id: i64,
) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "job");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "tablet");
    encode_i64(&mut buf, table_id);
    encode_i64(&mut buf, index_id);
    encode_i64(&mut buf, partition_id);
    encode_i64(&mut buf, tablet_id);
    buf.to_vec()
}

//==============================================================================
// Storage Vault Keys
//==============================================================================

/// 0x01 "storage_vault" ${instance_id} "vault" ${resource_id} -> StorageVaultPB
pub fn storage_vault_key(instance_id: &str, resource_id: &str) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    encode_str(&mut buf, "storage_vault");
    encode_str(&mut buf, instance_id);
    encode_str(&mut buf, "vault");
    encode_str(&mut buf, resource_id);
    buf.to_vec()
}

//==============================================================================
// System Keys
//==============================================================================

/// 0x02 "system" "meta-service" "registry" -> MetaServiceRegistryPB
pub fn meta_service_registry_key() -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_SYS_KEY_SPACE);
    encode_str(&mut buf, "system");
    encode_str(&mut buf, "meta-service");
    encode_str(&mut buf, "registry");
    buf.to_vec()
}

//==============================================================================
// Range Query Helpers
//==============================================================================

/// Get key prefix for range queries
pub fn key_prefix(parts: &[&str]) -> Vec<u8> {
    let mut buf = BytesMut::new();
    buf.put_u8(CLOUD_USER_KEY_SPACE);
    for part in parts {
        encode_str(&mut buf, part);
    }
    buf.to_vec()
}

/// Get key range for prefix scan
pub fn key_range_for_prefix(prefix: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut end = prefix.to_vec();
    // Increment the last byte for exclusive upper bound
    if let Some(last) = end.last_mut() {
        if *last == 0xff {
            // Handle overflow by extending
            end.push(0x00);
        } else {
            *last += 1;
        }
    }
    (prefix.to_vec(), end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_key() {
        let key = instance_key("test-instance");
        assert_eq!(key[0], CLOUD_USER_KEY_SPACE);
        assert!(key.len() > 1);
    }

    #[test]
    fn test_txn_keys() {
        let label_key = txn_label_key("inst1", 100, "label1");
        let info_key = txn_info_key("inst1", 100, 200);

        assert_eq!(label_key[0], CLOUD_USER_KEY_SPACE);
        assert_eq!(info_key[0], CLOUD_USER_KEY_SPACE);
        assert_ne!(label_key, info_key);
    }

    #[test]
    fn test_tablet_meta_key() {
        let key = tablet_meta_key("inst1", 1, 2, 3, 4);
        assert_eq!(key[0], CLOUD_USER_KEY_SPACE);
    }

    #[test]
    fn test_key_ordering() {
        let key1 = tablet_meta_key("inst1", 1, 2, 3, 100);
        let key2 = tablet_meta_key("inst1", 1, 2, 3, 200);

        // Keys should be lexicographically ordered
        assert!(key1 < key2);
    }

    #[test]
    fn test_key_range() {
        let prefix = b"test_prefix";
        let (start, end) = key_range_for_prefix(prefix);

        assert_eq!(start, prefix);
        assert!(end > start);
    }
}
