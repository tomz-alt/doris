# Pipeline Execution Model (VERSION_3) - Implementation Guide

**Status**: 🟡 Research Complete, Implementation Pending
**Date**: 2025-11-19
**Goal**: Adapt Rust FE to Doris VERSION_3 pipeline execution model

---

## 🎯 Current Status

### What Works (95%)
- ✅ Real tablet metadata discovery from Java FE
- ✅ TPlanFragment generation with OLAP_SCAN_NODE
- ✅ TScanRangeLocations building with real data
- ✅ gRPC connection to C++ BE (port 8060)
- ✅ VERSION_3 protocol accepted by BE

### What's Needed (5%)
- ⏳ TPipelineFragmentParamsList implementation
- ⏳ Thrift serialization for pipeline structures
- ⏳ Integration with BackendClient

### BE Error Message
```
BE returned error (code 6):
(127.0.0.1)[INTERNAL_ERROR]Invalid TPipelineFragmentParamsList!
```

**Root Cause**: BE expects TPipelineFragmentParamsList but we're sending TPlanFragment

---

## 📚 Java FE Reference Implementation

### File Locations
```
/home/user/doris/fe/fe-core/src/main/java/org/apache/doris/qe/Coordinator.java
  - Lines 3185-3350: toThrift() method
  - Shows how to build TPipelineFragmentParams

/home/user/doris/gensrc/thrift/PaloInternalService.thrift
  - Lines 50-95: TPipelineFragmentParams struct
  - Lines 96-102: TPipelineFragmentParamsList struct
  - Lines 42-49: TPipelineInstanceParams struct
```

### Key Java FE Code (Coordinator.java:3185+)

```java
Map<TNetworkAddress, TPipelineFragmentParams> toThrift(int backendNum) {
    Map<TNetworkAddress, TPipelineFragmentParams> res = new HashMap();
    TPlanFragment fragmentThrift = fragment.toThrift();

    for (FInstanceExecParam instanceExecParam : instanceExecParams) {
        if (!res.containsKey(instanceExecParam.host)) {
            TPipelineFragmentParams params = new TPipelineFragmentParams();

            // REQUIRED FIELDS
            params.setProtocolVersion(PaloInternalServiceVersion.V1);  // = 0
            params.setQueryId(queryId);                                 // TUniqueId
            params.setPerExchNumSenders(perExchNumSenders);             // Map<i32, i32>
            params.setFragment(fragmentThrift);                         // TPlanFragment
            params.setLocalParams(Lists.newArrayList());                // List<TPipelineInstanceParams>

            // OPTIONAL BUT RECOMMENDED
            params.setCoord(coordAddress);                              // TNetworkAddress
            params.setNumSenders(instanceExecParams.size());            // i32
            params.setDescTbl(descTable);                               // TDescriptorTable
            params.setQueryGlobals(queryGlobals);                       // TQueryGlobals
            params.setQueryOptions(queryOptions);                       // TQueryOptions

            res.put(instanceExecParam.host, params);
        }

        // Build per-instance parameters
        TPipelineInstanceParams localParams = new TPipelineInstanceParams();
        localParams.setFragmentInstanceId(instanceExecParam.instanceId);  // TUniqueId
        localParams.setPerNodeScanRanges(scanRanges);                     // Map<i32, List<TScanRangeParams>>
        localParams.setSenderId(i);                                       // i32
        localParams.setBackendNum(backendNum++);                          // i32

        params.getLocalParams().add(localParams);
    }

    return res;
}
```

---

## 🏗️ Thrift Structure Definitions

### TPipelineFragmentParamsList (Top-level)
```thrift
struct TPipelineFragmentParamsList {
    1: optional list<TPipelineFragmentParams> params_list;
}
```

### TPipelineFragmentParams (Per-backend)
```thrift
struct TPipelineFragmentParams {
    1: required PaloInternalServiceVersion protocol_version  // = 0 (V1)
    2: required Types.TUniqueId query_id
    3: optional i32 fragment_id
    4: required map<Types.TPlanNodeId, i32> per_exch_num_senders
    23: optional Planner.TPlanFragment fragment              // THE PLAN!
    24: list<TPipelineInstanceParams> local_params           // INSTANCES
    10: optional Types.TNetworkAddress coord
    8: optional i32 num_senders
}
```

### TPipelineInstanceParams (Per-instance)
```thrift
struct TPipelineInstanceParams {
    1: required Types.TUniqueId fragment_instance_id
    3: required map<Types.TPlanNodeId, list<TScanRangeParams>> per_node_scan_ranges
    4: optional i32 sender_id
    6: optional i32 backend_num
}
```

### TScanRangeParams
```thrift
struct TScanRangeParams {
    1: required TScanRange scan_range
    2: optional i32 volume_id
}
```

---

## 🔧 Implementation Plan

### Step 1: Add Thrift Structures to thrift_plan.rs

```rust
// Add to fe-planner/src/thrift_plan.rs

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TUniqueId {
    pub hi: i64,
    pub lo: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TScanRangeParams {
    pub scan_range: TScanRange,       // From scan_range_builder
    pub volume_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPipelineInstanceParams {
    pub fragment_instance_id: TUniqueId,
    pub per_node_scan_ranges: HashMap<i32, Vec<TScanRangeParams>>,
    pub sender_id: Option<i32>,
    pub backend_num: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPipelineFragmentParams {
    pub protocol_version: i32,                              // 0 = V1
    pub query_id: TUniqueId,
    pub per_exch_num_senders: HashMap<i32, i32>,
    pub fragment: TPlanFragment,
    pub local_params: Vec<TPipelineInstanceParams>,
    pub coord: Option<TNetworkAddress>,
    pub num_senders: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPipelineFragmentParamsList {
    pub params_list: Vec<TPipelineFragmentParams>,
}
```

### Step 2: Add Thrift Serialization (thrift_serialize.rs)

```rust
pub fn serialize_pipeline_params(
    params: &TPipelineFragmentParamsList
) -> Result<Vec<u8>> {
    let mut transport = TBufferChannel::with_capacity(0, 1024);
    let mut protocol = TCompactOutputProtocol::new(&mut transport);

    // Write TPipelineFragmentParamsList
    protocol.write_struct_begin(&TStructIdentifier::new("TPipelineFragmentParamsList"))?;

    // Field 1: params_list (list<TPipelineFragmentParams>)
    protocol.write_field_begin(&TFieldIdentifier::new("params_list", TType::List, 1))?;
    protocol.write_list_begin(&TListIdentifier::new(TType::Struct, params.params_list.len() as i32))?;

    for param in &params.params_list {
        write_pipeline_fragment_params(&mut protocol, param)?;
    }

    protocol.write_list_end()?;
    protocol.write_field_end()?;
    protocol.write_field_stop()?;
    protocol.write_struct_end()?;

    Ok(transport.write_bytes().to_vec())
}

fn write_pipeline_fragment_params<P: TOutputProtocol>(
    protocol: &mut P,
    params: &TPipelineFragmentParams
) -> Result<()> {
    protocol.write_struct_begin(&TStructIdentifier::new("TPipelineFragmentParams"))?;

    // Field 1: protocol_version (i32)
    protocol.write_field_begin(&TFieldIdentifier::new("protocol_version", TType::I32, 1))?;
    protocol.write_i32(params.protocol_version)?;
    protocol.write_field_end()?;

    // Field 2: query_id (TUniqueId)
    protocol.write_field_begin(&TFieldIdentifier::new("query_id", TType::Struct, 2))?;
    write_unique_id(protocol, &params.query_id)?;
    protocol.write_field_end()?;

    // Field 4: per_exch_num_senders (map<i32, i32>)
    protocol.write_field_begin(&TFieldIdentifier::new("per_exch_num_senders", TType::Map, 4))?;
    protocol.write_map_begin(&TMapIdentifier::new(TType::I32, TType::I32, params.per_exch_num_senders.len() as i32))?;
    for (k, v) in &params.per_exch_num_senders {
        protocol.write_i32(*k)?;
        protocol.write_i32(*v)?;
    }
    protocol.write_map_end()?;
    protocol.write_field_end()?;

    // Field 23: fragment (TPlanFragment)
    protocol.write_field_begin(&TFieldIdentifier::new("fragment", TType::Struct, 23))?;
    write_plan_fragment(protocol, &params.fragment)?;  // Reuse existing function
    protocol.write_field_end()?;

    // Field 24: local_params (list<TPipelineInstanceParams>)
    protocol.write_field_begin(&TFieldIdentifier::new("local_params", TType::List, 24))?;
    protocol.write_list_begin(&TListIdentifier::new(TType::Struct, params.local_params.len() as i32))?;
    for local_param in &params.local_params {
        write_pipeline_instance_params(protocol, local_param)?;
    }
    protocol.write_list_end()?;
    protocol.write_field_end()?;

    // Optional fields...
    if let Some(coord) = &params.coord {
        // Field 10: coord (TNetworkAddress)
        protocol.write_field_begin(&TFieldIdentifier::new("coord", TType::Struct, 10))?;
        write_network_address(protocol, coord)?;
        protocol.write_field_end()?;
    }

    if let Some(num_senders) = params.num_senders {
        // Field 8: num_senders (i32)
        protocol.write_field_begin(&TFieldIdentifier::new("num_senders", TType::I32, 8))?;
        protocol.write_i32(num_senders)?;
        protocol.write_field_end()?;
    }

    protocol.write_field_stop()?;
    protocol.write_struct_end()?;

    Ok(())
}
```

### Step 3: Build Pipeline Params from Scan Ranges

```rust
impl TPipelineFragmentParamsList {
    pub fn from_fragment_and_scan_ranges(
        fragment: TPlanFragment,
        query_id: [u8; 16],
        node_id: i32,
        scan_range_locations: &[TScanRangeLocations],
    ) -> Self {
        // Convert query_id to TUniqueId
        let unique_id = TUniqueId {
            hi: i64::from_be_bytes(query_id[0..8].try_into().unwrap()),
            lo: i64::from_be_bytes(query_id[8..16].try_into().unwrap()),
        };

        // Build scan range params
        let mut per_node_scan_ranges = HashMap::new();
        let scan_params: Vec<TScanRangeParams> = scan_range_locations
            .iter()
            .map(|loc| TScanRangeParams {
                scan_range: loc.scan_range.clone(),
                volume_id: None,
            })
            .collect();
        per_node_scan_ranges.insert(node_id, scan_params);

        // Build instance params
        let local_params = vec![TPipelineInstanceParams {
            fragment_instance_id: unique_id.clone(),
            per_node_scan_ranges,
            sender_id: Some(0),
            backend_num: Some(0),
        }];

        // Build fragment params
        let params = TPipelineFragmentParams {
            protocol_version: 0,  // V1
            query_id: unique_id,
            per_exch_num_senders: HashMap::new(),
            fragment,
            local_params,
            coord: None,
            num_senders: Some(1),
        };

        TPipelineFragmentParamsList {
            params_list: vec![params],
        }
    }
}
```

### Step 4: Update BackendClient (fe-backend-client/src/lib.rs)

```rust
pub async fn exec_plan_fragment_v3(
    &mut self,
    fragment: &TPlanFragment,
    query_id: [u8; 16],
    scan_ranges: &[TScanRangeLocations],
) -> Result<[u8; 16]> {
    // Build pipeline params
    let pipeline_params = TPipelineFragmentParamsList::from_fragment_and_scan_ranges(
        fragment.clone(),
        query_id,
        0,  // node_id for OLAP_SCAN_NODE
        scan_ranges,
    );

    // Serialize pipeline params to Thrift
    let pipeline_bytes = serialize_pipeline_params(&pipeline_params)?;

    // Create gRPC request
    let request = tonic::Request::new(PExecPlanFragmentRequest {
        request: Some(pipeline_bytes),
        compact: Some(true),
        version: Some(3),  // VERSION_3
    });

    // Send to BE
    let response = self.client.exec_plan_fragment(request).await?;

    // Handle response...
}
```

### Step 5: Update rust_fe_to_be_query.rs Example

```rust
// Change from:
let finst_id = client.exec_plan_fragment(&plan_fragment, query_id).await?;

// To:
let finst_id = client.exec_plan_fragment_v3(&plan_fragment, query_id, &scan_ranges).await?;
```

---

## ✅ Testing Strategy

### Test 1: Minimal Pipeline Params
```rust
#[test]
fn test_build_minimal_pipeline_params() {
    let fragment = TPlanFragment { /* ... */ };
    let query_id = [1u8; 16];
    let scan_ranges = vec![/* ... */];

    let params = TPipelineFragmentParamsList::from_fragment_and_scan_ranges(
        fragment,
        query_id,
        0,
        &scan_ranges,
    );

    assert_eq!(params.params_list.len(), 1);
    assert_eq!(params.params_list[0].protocol_version, 0);
}
```

### Test 2: Thrift Serialization
```rust
#[test]
fn test_serialize_pipeline_params() {
    let params = TPipelineFragmentParamsList { /* ... */ };
    let bytes = serialize_pipeline_params(&params).unwrap();
    assert!(!bytes.is_empty());
}
```

### Test 3: End-to-End with BE
```bash
$ cargo run --example rust_fe_to_be_query
# Should connect to BE and execute query successfully
```

---

## 📊 Expected BE Behavior

### Current (Failing)
```
Step 5: Executing Query on C++ BE
Error: InternalError("BE returned error (code 6): Invalid TPipelineFragmentParamsList!")
```

### After Implementation (Success)
```
Step 5: Executing Query on C++ BE
  Query ID: a1b2c3d4...
  ✓ Query submitted to BE
  Fragment Instance ID: a1b2c3d4...

Step 6: Fetching Results from BE (PBlock)
  ✓ Received 4 rows from BE

Step 7: Query Results
Row 1:
  [0]: 1
  [1]: 155190
  ... (16 columns total)
```

---

## 🎯 Success Criteria

- ✅ TPipelineFragmentParamsList builds correctly
- ✅ Thrift serialization produces valid binary
- ✅ BE accepts the request without error
- ✅ BE executes the query on real tablet data
- ✅ PBlock results are returned
- ✅ All 4 sample rows are retrieved
- ✅ End-to-end query works (MySQL JDBC → Rust FE → C++ BE)

---

## 📝 Notes for Next Session

1. **Priority**: Implement Thrift serialization for pipeline structures
2. **Reference**: Use existing `serialize_plan_fragment` as template
3. **Testing**: Build incrementally with unit tests
4. **Validation**: Compare with Java FE Thrift output if possible
5. **Fallback**: If Thrift serialization is too complex, consider using proto generation

---

## 🔗 Related Files

- **Current Implementation**: `rust_fe/fe-backend-client/examples/rust_fe_to_be_query.rs`
- **Thrift Definitions**: `/home/user/doris/gensrc/thrift/PaloInternalService.thrift`
- **Java Reference**: `/home/user/doris/fe/fe-core/src/main/java/org/apache/doris/qe/Coordinator.java`
- **Session Documentation**: `rust_fe/SESSION_E2E_QUERY_2025-11-19.md`

---

**Last Updated**: 2025-11-19
**Next Milestone**: Complete pipeline execution → Full TPC-H queries
