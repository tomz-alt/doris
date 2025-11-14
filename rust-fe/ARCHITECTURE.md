# Architecture Notes

## FE-BE Communication Protocol

### Current Doris Architecture

- **Backend (BE)**: Uses bRPC (Baidu RPC) to implement services
  - File: `be/src/service/internal_service.h`
  - Service: `PInternalService : public PBackendService`
  - Protocol: Protobuf-based, supports multiple protocols including gRPC

- **Frontend (FE)**: Uses Java gRPC client to connect to BE
  - File: `fe/fe-core/src/main/java/org/apache/doris/rpc/BackendServiceClient.java`
  - Uses: `io.grpc.ManagedChannel` and Netty
  - Connects via gRPC protocol that bRPC understands

### Why This Works

bRPC (Baidu RPC) is a multi-protocol RPC framework that supports:
- bRPC native protocol (Baidu protocol)
- gRPC protocol
- HTTP/HTTP2
- Thrift
- And others

So even though BE uses bRPC, FE can connect using standard gRPC because bRPC implements gRPC protocol compatibility.

### Rust FE Implementation

This Rust FE implementation uses:
- **tonic** (Rust gRPC library) for BE communication
- This works because bRPC on BE side understands gRPC protocol
- No need for a Rust bRPC client

### Alternative: Native bRPC Client

If you want to use native bRPC protocol instead of gRPC:

1. **No mature Rust bRPC client exists** (as of 2025)
2. **Options**:
   - Use gRPC (current approach with tonic) - **Recommended**
   - Implement bRPC protocol in Rust
   - Use FFI bindings to C++ bRPC library
   - Use HTTP/2 protocol that bRPC also supports

### Protocol Comparison

| Aspect | gRPC (tonic) | Native bRPC |
|--------|--------------|-------------|
| Rust Library | ✅ Mature (tonic) | ❌ None |
| Compatibility | ✅ Works with bRPC | ✅ Native |
| Performance | ⚡ Fast | ⚡ Very Fast |
| Implementation | ✅ Easy | ❌ Complex |
| Maintenance | ✅ Community | ❌ Custom |

**Recommendation**: Use gRPC (tonic) for production. The performance difference is negligible for FE use case, and you get better Rust ecosystem support.

## HTTP/2 Alternative

Another option is using HTTP/2 directly since bRPC supports it:

```rust
use reqwest::Client;

let client = Client::builder()
    .http2_prior_knowledge()
    .build()?;

// Make HTTP/2 requests to bRPC service
```

But this loses type safety of protobuf definitions, so gRPC is better.

## Summary

The current implementation using **gRPC (tonic)** is correct and will work seamlessly with Doris BE nodes running bRPC. No changes needed.
