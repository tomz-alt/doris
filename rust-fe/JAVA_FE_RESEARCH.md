# Java FE Research Findings - BE Communication

## Key Discovery: Two Different Data Fetch APIs

### 1. Legacy API (What Java FE Currently Uses)
- **Method**: `fetch_data()`
- **Request**: `PFetchDataRequest`
- **Response**: `PFetchDataResult`
  - `optional bytes row_batch = 5;` ← Legacy row-batch format
  - `optional bool empty_batch = 6;`
- **Location**: `ResultReceiver.java:58` uses `fetchDataAsyncWithCallback`

### 2. Modern Arrow API (What We Tried)
- **Method**: `fetch_arrow_data()`
- **Request**: `PFetchArrowDataRequest`
- **Response**: `PFetchArrowDataResult`
  - `optional PBlock block = 4;` ← Modern Arrow/PBlock format
  - `optional bool empty_batch = 5;`
- **Our implementation**: `src/be/client.rs:460-511`

## Java FE Flow (from Coordinator.java)

1. **Execute Fragments**:
   - `execPlanFragmentsAsync()` - Note: plural "Fragments"
   - Location: Line 3034 in Coordinator.java

2. **Fetch Results**:
   - Uses `ResultReceiver` class
   - Calls `fetchDataAsyncWithCallback()` → gets `PFetchDataResult`
   - Retrieves data in legacy `row_batch` format
   - Calls `getNext()` method repeatedly until EOS

3. **NOT using Arrow/PBlock**:
   - Java FE currently uses row-batch format
   - PBlock/Arrow API exists but may not be default

## Root Cause of Hang

**Hypothesis**: The BE might be configured to respond via the old `fetch_data` API, not `fetch_arrow_data`.

### Evidence:
1. We call `exec_pipeline_fragments()`
2. We then call `fetch_arrow_data()`
3. But BE might be waiting for `fetch_data()` call instead
4. This causes the hang - neither side is calling the right method

## Potential Solutions

### Option 1: Use Legacy fetch_data() API
- Implement `fetch_data()` method
- Parse `row_batch` bytes (Thrift TResultBatch format)
- This is what Java FE does successfully

### Option 2: Configure BE for Arrow/PBlock
- Find the configuration/flag that tells BE to use PBlock
- Might be in fragment execution parameters
- Might be a session variable or BE config

### Option 3: Hybrid Approach
- Try `fetch_data()` first (proven to work)
- Later migrate to `fetch_arrow_data()` when we understand the trigger

## Next Steps

1. **Implement fetch_data() with row_batch parsing**
   - This is the proven path that Java FE uses
   - Should unblock mysql CLI immediately

2. **Research PBlock trigger**
   - Find what tells BE to use PBlock vs row_batch
   - Likely in `TPlanFragmentExecParams` or similar

3. **Migrate to PBlock later**
   - Once row_batch works, we can add PBlock support
   - This is a performance optimization, not a requirement

## Code References

- Java Coordinator: `/Users/zhhanz/Documents/velodb/doris/fe/fe-core/src/main/java/org/apache/doris/qe/Coordinator.java`
- Java ResultReceiver: `/Users/zhhanz/Documents/velodb/doris/fe/fe-core/src/main/java/org/apache/doris/qe/ResultReceiver.java`
- Proto definitions: `rust-fe/proto/internal_service.proto`
