# Rust FE ↔ C++ BE Communication Guide

## Prerequisites

**IMPORTANT**: Protobuf compiler (protoc) is required for production use:

```bash
# Debian/Ubuntu
sudo apt-get install protobuf-compiler

# macOS
brew install protobuf

# Verify installation
protoc --version  # Should be 3.x or later
```

**⚠️ INSTALLATION BLOCKED**: See [PROTOC_INSTALLATION.md](PROTOC_INSTALLATION.md) for details and workarounds:
- Manual installation with privileged access
- Pre-generated bindings (recommended for CI/CD)
- Docker container with protoc
- MockBackend for development (current - 200 tests passing)

## Architecture Overview

Doris uses **gRPC with Protobuf** for Frontend-Backend communication.

```
┌─────────────┐
│ MySQL CLI   │
└──────┬──────┘
       │ MySQL Protocol (port 9030)
       ▼
┌─────────────┐         ┌─────────────┐
│   Rust FE   │  gRPC   │   C++ BE    │
│             │◄───────►│             │
│             │Protobuf │  (port 9060)│
│ - Parse SQL │         │             │
│ - Plan query│         │ - Store data│
│ - Metadata  │         │ - Execute   │
└─────────────┘         └─────────────┘
```

## gRPC Service Definition

From `gensrc/proto/internal_service.proto`:

```protobuf
service PBackendService {
    // Execute a query fragment
    rpc exec_plan_fragment(PExecPlanFragmentRequest)
        returns (PExecPlanFragmentResult);

    // Fetch result data
    rpc fetch_data(PFetchDataRequest)
        returns (PFetchDataResult);

    // Cancel running query
    rpc cancel_plan_fragment(PCancelPlanFragmentRequest)
        returns (PCancelPlanFragmentResult);
}
```

## Key Messages

### PExecPlanFragmentRequest
```protobuf
message PExecPlanFragmentRequest {
    optional bytes request = 1;    // Serialized TPlanFragmentParams
    optional bool compact = 2;      // Use compact serialization
    optional PFragmentRequestVersion version = 3;
}
```

The `request` field contains serialized Thrift `TPlanFragmentParams`.

### PExecPlanFragmentResult
```protobuf
message PExecPlanFragmentResult {
    required PStatus status = 1;
    optional int64 received_time = 2;
    optional int64 execution_time = 3;
}
```

### PFetchDataRequest
```protobuf
message PFetchDataRequest {
    required PUniqueId finst_id = 1;  // Fragment instance ID
    optional bool resp_in_attachment = 2;
}
```

### PFetchDataResult
```protobuf
message PFetchDataResult {
    required PStatus status = 1;
    optional int64 packet_seq = 2;
    optional bool eos = 3;              // End of stream
    optional bytes row_batch = 5;       // Serialized row data
    optional bool empty_batch = 6;
}
```

## Implementation Steps

### 1. Generate Protobuf Bindings

Use `prost` and `tonic` to generate Rust bindings:

```toml
[build-dependencies]
tonic-build = "0.10"
prost-build = "0.12"
```

```rust
// build.rs
fn main() {
    tonic_build::configure()
        .compile(
            &["gensrc/proto/internal_service.proto"],
            &["gensrc/proto"],
        )
        .unwrap();
}
```

### 2. Create fe-backend-client Crate

```rust
// fe-backend-client/src/lib.rs
use tonic::transport::Channel;
use prost::Message;

pub struct BackendClient {
    client: PBackendServiceClient<Channel>,
    be_address: String,
}

impl BackendClient {
    pub async fn new(be_host: &str, be_port: u16) -> Result<Self> {
        let addr = format!("http://{}:{}", be_host, be_port);
        let client = PBackendServiceClient::connect(addr).await?;

        Ok(Self {
            client,
            be_address: format!("{}:{}", be_host, be_port),
        })
    }

    pub async fn exec_plan_fragment(
        &mut self,
        fragment: TPlanFragment,
        query_id: TUniqueId,
    ) -> Result<PExecPlanFragmentResult> {
        // 1. Serialize fragment to Thrift bytes
        let fragment_bytes = serialize_thrift(&fragment)?;

        // 2. Create gRPC request
        let request = PExecPlanFragmentRequest {
            request: Some(fragment_bytes),
            compact: Some(true),
            version: Some(PFragmentRequestVersion::Version2),
        };

        // 3. Send RPC
        let response = self.client
            .exec_plan_fragment(request)
            .await?;

        Ok(response.into_inner())
    }

    pub async fn fetch_data(
        &mut self,
        finst_id: PUniqueId,
    ) -> Result<Vec<Row>> {
        let request = PFetchDataRequest {
            finst_id: Some(finst_id),
            resp_in_attachment: Some(false),
        };

        let mut rows = Vec::new();
        loop {
            let response = self.client.fetch_data(request.clone()).await?;
            let result = response.into_inner();

            if let Some(row_batch) = result.row_batch {
                // Deserialize row_batch
                rows.extend(deserialize_rows(&row_batch)?);
            }

            if result.eos.unwrap_or(false) {
                break;
            }
        }

        Ok(rows)
    }
}
```

### 3. Update QueryExecutor

```rust
// fe-qe/src/executor.rs
use fe_backend_client::BackendClient;

pub struct QueryExecutor {
    catalog: Arc<Catalog>,
    be_client: Option<BackendClient>,  // NEW
}

impl QueryExecutor {
    pub async fn execute_select(&self, stmt: &SelectStatement)
        -> Result<QueryResult>
    {
        if let Some(ref client) = self.be_client {
            // 1. Create query plan
            let planner = QueryPlanner::new(self.catalog.clone());
            let plan = planner.plan(&Statement::Select(stmt.clone()))?;

            // 2. Generate query ID
            let query_id = generate_query_id();

            // 3. Send to BE
            let result = client.exec_plan_fragment(plan, query_id).await?;

            // 4. Fetch results
            let rows = client.fetch_data(result.finst_id).await?;

            // 5. Convert to ResultSet
            Ok(QueryResult::ResultSet(ResultSet {
                columns: extract_columns(&plan),
                rows,
            }))
        } else {
            // Fall back to schema-only execution
            self.execute_select_schema_only(stmt)
        }
    }
}
```

## Dependencies

Add to workspace `Cargo.toml`:

```toml
[workspace.dependencies]
tonic = "0.10"
prost = "0.12"
```

## Testing Strategy

1. **Unit Tests**: Mock BE responses
2. **Integration Tests**: Connect to real C++ BE
3. **Comparison Tests**: Java FE vs Rust FE results

## Next Steps

1. ✅ Document gRPC/Protobuf requirements (this file)
2. ⏸️ Generate protobuf bindings
3. ⏸️ Create fe-backend-client crate
4. ⏸️ Implement exec_plan_fragment RPC
5. ⏸️ Implement fetch_data RPC
6. ⏸️ Update QueryExecutor to use BE client
7. ⏸️ Test with real C++ BE
8. ⏸️ Verify identical results to Java FE
