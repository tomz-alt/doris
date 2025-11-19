# CRITICAL BUG FOUND: Missing REQUIRED partition Field

## Discovery Date: 2025-11-19

## Symptom
BE returns "TProtocolException: Invalid data" when deserializing TPipelineFragmentParams from Rust FE.

## Root Cause

**TPlanFragment is missing the REQUIRED `partition` field!**

### Thrift Specification

From `/home/user/doris/gensrc/thrift/Planner.thrift:31-67`:

```thrift
struct TPlanFragment {
  2: optional PlanNodes.TPlan plan
  6: required Partitions.TDataPartition partition  ← REQUIRED!!!
}
```

### Our Incorrect Implementation

`/home/user/doris/rust_fe/fe-planner/src/thrift_plan.rs:141-143`:

```rust
pub struct TPlanFragment {
    pub plan: TPlan,     ← Only has plan
    // MISSING: partition field!
}
```

### Our Incorrect Serialization

`/home/user/doris/rust_fe/fe-planner/src/thrift_serialize.rs:609-634`:

```rust
fn write_plan_fragment<P: TOutputProtocol>(
    protocol: &mut P,
    fragment: &TPlanFragment,
) -> Result<()> {
    protocol.write_struct_begin(&TStructIdentifier::new("TPlanFragment"))?;

    // Field 1: plan (WRONG FIELD ID - should be 2!)
    protocol.write_field_begin(&TFieldIdentifier::new("plan", TType::Struct, 1))?;
    write_plan(protocol, &fragment.plan)?;
    protocol.write_field_end()?;

    // MISSING: Field 6 partition (REQUIRED!)

    protocol.write_field_stop()?;
    protocol.write_struct_end()?;
    Ok(())
}
```

## Additional Issues Found

1. **Wrong field ID**: We write `plan` as field 1, but .thrift shows it's field 2
2. **Missing required field**: No `partition` field (field 6, REQUIRED)

## TDataPartition Structure

From `/home/user/doris/gensrc/thrift/Partitions.thrift:95-99`:

```thrift
struct TDataPartition {
  1: required TPartitionType type           ← REQUIRED
  2: optional list<Exprs.TExpr> partition_exprs
  3: optional list<TRangePartition> partition_infos
}

enum TPartitionType {
  UNPARTITIONED = 0,
  RANDOM = 1,
  HASH_PARTITIONED = 2,
  RANGE_PARTITIONED = 3,
  LIST_PARTITIONED = 4,
  BUCKET_SHFFULE_HASH_PARTITIONED = 5,
  OLAP_TABLE_SINK_HASH_PARTITIONED = 6,
  HIVE_TABLE_SINK_HASH_PARTITIONED = 7,
}
```

## Required Fix

### 1. Add TDataPartition to thrift_plan.rs

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TPartitionType {
    Unpartitioned = 0,
    Random = 1,
    HashPartitioned = 2,
    RangePartitioned = 3,
    ListPartitioned = 4,
    BucketShuffleHashPartitioned = 5,
    OlapTableSinkHashPartitioned = 6,
    HiveTableSinkHashPartitioned = 7,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TDataPartition {
    pub partition_type: TPartitionType,
    pub partition_exprs: Option<Vec<TExpr>>,  // For now, TExpr is a placeholder
    pub partition_infos: Option<Vec<()>>,      // TRangePartition not needed yet
}
```

### 2. Update TPlanFragment structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPlanFragment {
    pub plan: TPlan,
    pub partition: TDataPartition,  ← ADD THIS
}
```

### 3. Fix write_plan_fragment function

```rust
fn write_plan_fragment<P: TOutputProtocol>(
    protocol: &mut P,
    fragment: &TPlanFragment,
) -> Result<()> {
    protocol.write_struct_begin(&TStructIdentifier::new("TPlanFragment"))?;

    // Field 2: plan (TPlan) - CORRECTED FIELD ID
    protocol.write_field_begin(&TFieldIdentifier::new("plan", TType::Struct, 2))?;
    write_plan(protocol, &fragment.plan)?;
    protocol.write_field_end()?;

    // Field 6: partition (TDataPartition) - ADD THIS
    protocol.write_field_begin(&TFieldIdentifier::new("partition", TType::Struct, 6))?;
    write_data_partition(protocol, &fragment.partition)?;
    protocol.write_field_end()?;

    protocol.write_field_stop()?;
    protocol.write_struct_end()?;
    Ok(())
}

fn write_data_partition<P: TOutputProtocol>(
    protocol: &mut P,
    partition: &TDataPartition,
) -> Result<()> {
    protocol.write_struct_begin(&TStructIdentifier::new("TDataPartition"))?;

    // Field 1: type (TPartitionType) - REQUIRED
    protocol.write_field_begin(&TFieldIdentifier::new("type", TType::I32, 1))?;
    protocol.write_i32(partition.partition_type as i32)?;
    protocol.write_field_end()?;

    // Field 2: partition_exprs (optional)
    if let Some(ref exprs) = partition.partition_exprs {
        // Write partition_exprs if needed
    }

    // Field 3: partition_infos (optional)
    if let Some(ref infos) = partition.partition_infos {
        // Write partition_infos if needed
    }

    protocol.write_field_stop()?;
    protocol.write_struct_end()?;
    Ok(())
}
```

### 4. Update all TPlanFragment creation sites

For minimal queries like "SELECT 1", use:

```rust
let fragment = TPlanFragment {
    plan: TPlan { nodes: vec![] },
    partition: TDataPartition {
        partition_type: TPartitionType::Unpartitioned,
        partition_exprs: None,
        partition_infos: None,
    },
};
```

## Impact

This bug makes **100% of queries fail** because every query requires a TPlanFragment with a partition field.

## Priority

**CRITICAL** - Blocks all query execution

## Verification

After fixing, the hex dump should show:
1. Field 2 (plan) written correctly
2. Field 6 (partition) written with TPartitionType value
3. BE should accept the serialized bytes without "Invalid data" error

## Related Files

- `/home/user/doris/rust_fe/fe-planner/src/thrift_plan.rs` - Add TDataPartition, TPartitionType
- `/home/user/doris/rust_fe/fe-planner/src/thrift_serialize.rs` - Fix write_plan_fragment, add write_data_partition
- `/home/user/doris/rust_fe/fe-planner/src/planner.rs` - Update fragment creation
- `/home/user/doris/rust_fe/fe-planner/examples/thrift_hex_dump.rs` - Update test

## Testing

After fix:
1. Run `cargo run --example thrift_hex_dump`
2. Verify field 6 appears in output
3. Test with actual BE deserialization
4. Verify BE accepts without errors

## Lesson Learned

**Always check ALL fields in .thrift definitions, not just the ones that seem important!**

The word "required" in Thrift means the field MUST be present in serialization, even if logically it seems optional for simple queries.
