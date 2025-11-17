# Rust FE Migration TODO List

## Overview
This document tracks the migration of Apache Doris Frontend (FE) from Java to Rust.
- **Total Java Files**: 4,654
- **Major Modules**: 48
- **Status**: In Progress

## Migration Strategy
1. **Phase 1**: Infrastructure & Common Utilities
2. **Phase 2**: Core Data Structures & Catalog
3. **Phase 3**: SQL Parsing & Analysis
4. **Phase 4**: Query Optimization & Planning
5. **Phase 5**: Execution Engine & Services
6. **Phase 6**: Advanced Features & Extensions

---

## Phase 1: Infrastructure & Common Utilities (Priority: CRITICAL)

### 1.1 Project Setup
- [ ] Create rust_fe workspace directory structure
- [ ] Set up Cargo.toml with workspace configuration
- [ ] Configure build system and dependencies
- [ ] Set up testing framework
- [ ] Configure CI/CD for Rust FE
- [ ] Set up logging framework (equivalent to log4j)
- [ ] Configure error handling patterns

### 1.2 Common Module (~200 files)
- [ ] Migrate basic types and constants
- [ ] Migrate exception/error types to Rust Result<T, E>
- [ ] Migrate configuration management (Config.java)
- [ ] Migrate utility classes (StringUtils, etc.)
- [ ] Migrate version management
- [ ] Migrate authentication/authorization basics
- [ ] Migrate common data structures
- [ ] Migrate network utilities
- [ ] Migrate serialization/deserialization framework
- [ ] Migrate metrics collection basics

### 1.3 RPC Module (~50 files)
- [ ] Set up Thrift/gRPC bindings for Rust
- [ ] Migrate RPC client implementations
- [ ] Migrate RPC server implementations
- [ ] Migrate service interfaces
- [ ] Implement async RPC handlers

---

## Phase 2: Core Data Structures & Catalog (Priority: HIGH)

### 2.1 Catalog Module (~300 files)
- [ ] Migrate database metadata structures (Database.java)
- [ ] Migrate table metadata structures (Table.java)
- [ ] Migrate column definitions (Column.java)
- [ ] Migrate partition information (Partition.java)
- [ ] Migrate index definitions
- [ ] Migrate materialized view metadata
- [ ] Migrate schema change handlers
- [ ] Migrate catalog manager
- [ ] Migrate metadata persistence
- [ ] Migrate catalog operations (CREATE/DROP/ALTER)
- [ ] Migrate table types (OLAP, MySQL, Elasticsearch, etc.)
- [ ] Migrate replica management
- [ ] Migrate tablet metadata

### 2.2 Persist Module (~100 files)
- [ ] Migrate persistence layer interfaces
- [ ] Migrate journal system
- [ ] Migrate checkpoint mechanism
- [ ] Migrate metadata serialization
- [ ] Migrate image read/write
- [ ] Migrate edit log operations
- [ ] Migrate metadata recovery

### 2.3 System Module (~80 files)
- [ ] Migrate backend (BE) information management
- [ ] Migrate frontend (FE) information management
- [ ] Migrate cluster management
- [ ] Migrate node heartbeat mechanism
- [ ] Migrate system info tables
- [ ] Migrate health check mechanism

---

## Phase 3: SQL Parsing & Analysis (Priority: HIGH)

### 3.1 Analysis Module (~400 files)
- [ ] Set up SQL parser (consider using sqlparser-rs or custom)
- [ ] Migrate AST node types
- [ ] Migrate expression analysis (Expr.java and subclasses)
- [ ] Migrate statement analysis (StatementBase.java)
- [ ] Migrate SELECT statement analysis
- [ ] Migrate INSERT statement analysis
- [ ] Migrate UPDATE/DELETE statement analysis
- [ ] Migrate DDL statement analysis
- [ ] Migrate DML statement analysis
- [ ] Migrate function call analysis
- [ ] Migrate subquery analysis
- [ ] Migrate join analysis
- [ ] Migrate aggregate function analysis
- [ ] Migrate window function analysis
- [ ] Migrate type coercion rules
- [ ] Migrate semantic validation
- [ ] Migrate privilege checking during analysis

### 3.2 Rewrite Module (~50 files)
- [ ] Migrate expression rewrite rules
- [ ] Migrate predicate pushdown
- [ ] Migrate constant folding
- [ ] Migrate expression simplification
- [ ] Migrate subquery rewrite

---

## Phase 4: Query Optimization & Planning (Priority: HIGH)

### 4.1 Nereids Module - New Optimizer (~800 files)
**This is a large and critical module**
- [ ] Migrate Nereids core framework
- [ ] Migrate pattern matching system
- [ ] Migrate rule-based optimizer
- [ ] Migrate cost-based optimizer
- [ ] Migrate statistics framework
- [ ] Migrate memo structure
- [ ] Migrate group/group expression
- [ ] Migrate properties system
- [ ] Migrate plan nodes (LogicalPlan)
- [ ] Migrate physical plan nodes
- [ ] Migrate plan transformations
- [ ] Migrate join reorder
- [ ] Migrate aggregate optimization
- [ ] Migrate window function optimization
- [ ] Migrate materialized view selection
- [ ] Migrate partition pruning
- [ ] Migrate runtime filter generation
- [ ] Migrate plan cost estimation

### 4.2 Planner Module - Legacy Planner (~200 files)
- [ ] Migrate PlanNode hierarchy
- [ ] Migrate ScanNode implementations
- [ ] Migrate JoinNode implementations
- [ ] Migrate AggregationNode
- [ ] Migrate SortNode
- [ ] Migrate ExchangeNode
- [ ] Migrate UnionNode
- [ ] Migrate plan fragments
- [ ] Migrate distributed plan generation
- [ ] Migrate plan cost model
- [ ] Migrate data partition strategies

### 4.3 Statistics Module (~150 files)
- [ ] Migrate statistics collection
- [ ] Migrate histogram support
- [ ] Migrate column statistics
- [ ] Migrate table statistics
- [ ] Migrate statistics cache
- [ ] Migrate auto-analyze
- [ ] Migrate cardinality estimation

---

## Phase 5: Execution Engine & Services (Priority: HIGH)

### 5.1 QE (Query Execution) Module (~200 files)
- [ ] Migrate ConnectContext
- [ ] Migrate SessionVariable
- [ ] Migrate query state machine
- [ ] Migrate coordinator (query execution coordinator)
- [ ] Migrate result set handling
- [ ] Migrate query cache
- [ ] Migrate prepared statement support
- [ ] Migrate transaction context
- [ ] Migrate query timeout handling
- [ ] Migrate result sink

### 5.2 Service Module (~100 files)
- [ ] Migrate FrontendService (main service interface)
- [ ] Migrate ExecuteService
- [ ] Migrate HTTP service handlers
- [ ] Migrate MySQL protocol implementation
- [ ] Migrate query handler
- [ ] Migrate connection management
- [ ] Migrate session management
- [ ] Migrate authentication service
- [ ] Migrate authorization service

### 5.3 MySQL Protocol Module (~80 files)
- [ ] Migrate MySQL protocol codec
- [ ] Migrate MySQL packet handlers
- [ ] Migrate MySQL authentication
- [ ] Migrate result set encoding
- [ ] Migrate prepared statement protocol
- [ ] Migrate command handlers

### 5.4 HTTP v2 Module (~120 files)
- [ ] Migrate REST API endpoints
- [ ] Migrate HTTP handlers
- [ ] Migrate query API
- [ ] Migrate load API
- [ ] Migrate admin API
- [ ] Migrate metadata API
- [ ] Migrate health check endpoints

---

## Phase 6: Data Loading & Management (Priority: MEDIUM)

### 6.1 Load Module (~150 files)
- [ ] Migrate load job management
- [ ] Migrate broker load
- [ ] Migrate stream load
- [ ] Migrate routine load
- [ ] Migrate insert load
- [ ] Migrate load scheduler
- [ ] Migrate load progress tracking
- [ ] Migrate load state management
- [ ] Migrate error handling in load

### 6.2 Transaction Module (~80 files)
- [ ] Migrate transaction manager
- [ ] Migrate transaction state
- [ ] Migrate 2PC protocol
- [ ] Migrate transaction coordinator
- [ ] Migrate commit/rollback logic
- [ ] Migrate transaction cleanup
- [ ] Migrate transaction timeout handling

### 6.3 Backup Module (~60 files)
- [ ] Migrate backup job
- [ ] Migrate restore job
- [ ] Migrate snapshot management
- [ ] Migrate backup scheduler
- [ ] Migrate repository abstraction
- [ ] Migrate backup storage integration

---

## Phase 7: Advanced Features (Priority: MEDIUM)

### 7.1 Alter Module (~100 files)
- [ ] Migrate schema change
- [ ] Migrate rollup jobs
- [ ] Migrate materialized view creation
- [ ] Migrate partition operations
- [ ] Migrate column operations
- [ ] Migrate index operations
- [ ] Migrate table property modification

### 7.2 Clone Module (~40 files)
- [ ] Migrate tablet clone mechanism
- [ ] Migrate replica repair
- [ ] Migrate data migration
- [ ] Migrate tablet balancing

### 7.3 Datasource Module (~200 files)
- [ ] Migrate external table support
- [ ] Migrate JDBC catalog
- [ ] Migrate Hive catalog
- [ ] Migrate Iceberg catalog
- [ ] Migrate Hudi catalog
- [ ] Migrate Elasticsearch catalog
- [ ] Migrate MySQL catalog
- [ ] Migrate ODBC catalog
- [ ] Migrate external file systems (S3, HDFS, etc.)

### 7.4 FS (File System) Module (~80 files)
- [ ] Migrate file system abstraction
- [ ] Migrate HDFS support
- [ ] Migrate S3 support
- [ ] Migrate local file system
- [ ] Migrate BOS support
- [ ] Migrate OSS support
- [ ] Migrate COS support

---

## Phase 8: Cluster Management & Monitoring (Priority: MEDIUM)

### 8.1 HA (High Availability) Module (~40 files)
- [ ] Migrate leader election (using etcd or similar)
- [ ] Migrate BDBJE replacement (metadata storage)
- [ ] Migrate master/follower synchronization
- [ ] Migrate failover mechanism
- [ ] Migrate checkpoint sync

### 8.2 Master Module (~60 files)
- [ ] Migrate master daemon threads
- [ ] Migrate metadata checkpoint thread
- [ ] Migrate replica checker
- [ ] Migrate tablet scheduler
- [ ] Migrate load scheduler
- [ ] Migrate routine load scheduler
- [ ] Migrate statistics collector

### 8.3 Metric Module (~50 files)
- [ ] Migrate metrics collection
- [ ] Migrate Prometheus integration
- [ ] Migrate system metrics
- [ ] Migrate query metrics
- [ ] Migrate load metrics
- [ ] Migrate JVM metrics equivalent (Rust runtime metrics)

### 8.4 Monitor Module (~30 files)
- [ ] Migrate monitoring framework
- [ ] Migrate alerting
- [ ] Migrate health monitoring
- [ ] Migrate resource monitoring

---

## Phase 9: Job & Task Management (Priority: MEDIUM)

### 9.1 Job Module (~100 files)
- [ ] Migrate job framework
- [ ] Migrate job scheduler
- [ ] Migrate job execution
- [ ] Migrate job persistence
- [ ] Migrate various job types

### 9.2 Task Module (~80 files)
- [ ] Migrate task framework
- [ ] Migrate agent task
- [ ] Migrate backup task
- [ ] Migrate schema change task
- [ ] Migrate clone task
- [ ] Migrate upload task

### 9.3 Scheduler Module (~50 files)
- [ ] Migrate job scheduler
- [ ] Migrate task scheduler
- [ ] Migrate timer-based scheduling
- [ ] Migrate dependency-based scheduling

---

## Phase 10: Specialized Features (Priority: LOW-MEDIUM)

### 10.1 MTMV (Materialized View) Module (~120 files)
- [ ] Migrate materialized view management
- [ ] Migrate MV refresh
- [ ] Migrate MV query rewrite
- [ ] Migrate MV metadata
- [ ] Migrate MV scheduler

### 10.2 PL/SQL Module (~80 files)
- [ ] Migrate PL/SQL parser
- [ ] Migrate stored procedure support
- [ ] Migrate PL/SQL execution
- [ ] Migrate cursor support
- [ ] Migrate exception handling in PL/SQL

### 10.3 Table Function Module (~40 files)
- [ ] Migrate table function framework
- [ ] Migrate built-in table functions
- [ ] Migrate numbers() function
- [ ] Migrate explode functions
- [ ] Migrate JSON table functions

### 10.4 Dictionary Module (~30 files)
- [ ] Migrate dictionary encoding
- [ ] Migrate dictionary management
- [ ] Migrate dictionary cache

### 10.5 Binlog Module (~40 files)
- [ ] Migrate binlog framework
- [ ] Migrate binlog generation
- [ ] Migrate binlog consumption
- [ ] Migrate CDC support

### 10.6 Encryption Module (~20 files)
- [ ] Migrate encryption framework
- [ ] Migrate key management
- [ ] Migrate data encryption
- [ ] Migrate SSL/TLS support

### 10.7 Event Module (~20 files)
- [ ] Migrate event system
- [ ] Migrate event listeners
- [ ] Migrate event dispatch

---

## Phase 11: Policy & Security (Priority: MEDIUM)

### 11.1 Policy Module (~60 files)
- [ ] Migrate storage policy
- [ ] Migrate resource policy
- [ ] Migrate row policy
- [ ] Migrate data mask policy

### 11.2 Resource Module (~50 files)
- [ ] Migrate resource management
- [ ] Migrate resource group
- [ ] Migrate workload group
- [ ] Migrate query queue

### 11.3 Block Rule Module (~20 files)
- [ ] Migrate SQL block rules
- [ ] Migrate partition block rules

---

## Phase 12: Cloud & Modern Features (Priority: LOW-MEDIUM)

### 12.1 Cloud Module (~150 files)
- [ ] Migrate cloud-native features
- [ ] Migrate compute-storage separation
- [ ] Migrate cloud metadata service
- [ ] Migrate cloud transaction manager
- [ ] Migrate cloud-specific optimizations

### 12.2 Cooldown Module (~30 files)
- [ ] Migrate data cooldown
- [ ] Migrate tiered storage
- [ ] Migrate storage migration

### 12.3 InsertOverwrite Module (~30 files)
- [ ] Migrate insert overwrite logic
- [ ] Migrate atomic replace

---

## Phase 13: Plugin & Extension (Priority: LOW)

### 13.1 Plugin Module (~50 files)
- [ ] Migrate plugin framework
- [ ] Migrate plugin loader
- [ ] Migrate plugin lifecycle
- [ ] Migrate audit plugin
- [ ] Migrate custom plugin support

### 13.2 SPI Module (~30 files)
- [ ] Migrate service provider interface
- [ ] Migrate extensibility framework

---

## Phase 14: Utilities & Misc (Priority: LOW)

### 14.1 Deploy Module (~20 files)
- [ ] Migrate deployment utilities
- [ ] Migrate environment setup

### 14.2 Info Module (~20 files)
- [ ] Migrate system information
- [ ] Migrate version info

### 14.3 Consistency Module (~20 files)
- [ ] Migrate consistency checker
- [ ] Migrate replica consistency

### 14.4 Index Policy Module (~20 files)
- [ ] Migrate index policies
- [ ] Migrate automatic indexing

---

## Phase 15: Testing & Validation (Priority: CRITICAL)

### 15.1 Unit Tests
- [ ] Migrate unit tests for each module
- [ ] Achieve >80% code coverage
- [ ] Set up property-based testing

### 15.2 Integration Tests
- [ ] Create end-to-end tests
- [ ] Create query tests
- [ ] Create load tests
- [ ] Create HA tests
- [ ] Create performance regression tests

### 15.3 Compatibility Tests
- [ ] Test Java FE <-> Rust FE compatibility
- [ ] Test protocol compatibility
- [ ] Test metadata compatibility
- [ ] Test upgrade path (Java -> Rust)

---

## Phase 16: Performance & Optimization (Priority: HIGH)

### 16.1 Performance Benchmarking
- [ ] Establish baseline performance metrics
- [ ] Compare Rust FE vs Java FE performance
- [ ] Optimize hot paths
- [ ] Reduce memory allocation
- [ ] Optimize async/await usage

### 16.2 Memory Management
- [ ] Profile memory usage
- [ ] Optimize data structure layouts
- [ ] Implement efficient caching strategies
- [ ] Minimize clone operations

### 16.3 Concurrency Optimization
- [ ] Optimize lock contention
- [ ] Use lock-free data structures where appropriate
- [ ] Optimize async task scheduling

---

## Phase 17: Documentation & Migration (Priority: CRITICAL)

### 17.1 Documentation
- [ ] Write Rust FE architecture documentation
- [ ] Document API changes
- [ ] Create migration guide (Java -> Rust)
- [ ] Document configuration changes
- [ ] Create deployment guide

### 17.2 Tooling
- [ ] Create migration helper tools
- [ ] Create code generation tools for boilerplate
- [ ] Create testing utilities

### 17.3 Deployment Strategy
- [ ] Plan staged rollout
- [ ] Create rollback procedures
- [ ] Define feature flags for gradual migration
- [ ] Create monitoring dashboards

---

## Progress Tracking

### Overall Statistics
- **Total Modules**: 48
- **Estimated Files to Migrate**: ~4,654
- **Completed Modules**: 0
- **In Progress Modules**: 0
- **Completion Percentage**: 0%

### Critical Path Items (Must Complete First)
1. ✗ Common module
2. ✗ Catalog module
3. ✗ Analysis module
4. ✗ Nereids/Planner modules
5. ✗ QE module
6. ✗ Service module

### Dependencies Graph
```
Common (Phase 1)
├── Catalog (Phase 2)
│   ├── Analysis (Phase 3)
│   │   ├── Nereids (Phase 4)
│   │   └── Planner (Phase 4)
│   │       └── QE (Phase 5)
│   │           └── Service (Phase 5)
│   ├── Persist (Phase 2)
│   └── System (Phase 2)
├── RPC (Phase 1)
└── Statistics (Phase 4)
```

---

## Notes & Decisions

### Technology Choices
- **Parser**: TBD - sqlparser-rs vs custom ANTLR-based parser
- **Async Runtime**: Tokio
- **Serialization**: serde + bincode/protobuf
- **RPC**: Tonic (gRPC) or rust-thrift
- **Metadata Storage**: TBD - replace BDBJE with etcd or similar
- **HTTP Framework**: axum or actix-web
- **MySQL Protocol**: custom implementation or mysql-async fork

### Key Challenges
1. Java's garbage collection vs Rust's ownership model
2. Massive codebase size (4,654 files)
3. Complex SQL optimization logic in Nereids
4. Thread safety guarantees
5. Maintaining compatibility during migration
6. Performance regression prevention
7. Team training on Rust

### Success Criteria
1. Feature parity with Java FE
2. Performance improvement (target: 20-30% better)
3. Memory usage reduction (target: 40-50% less)
4. Zero downtime migration capability
5. Full test coverage
6. Production-ready stability

---

## Timeline Estimation

### Aggressive Estimate
- **Phase 1**: 2 months
- **Phase 2**: 3 months
- **Phase 3**: 3 months
- **Phase 4**: 6 months (Nereids is complex)
- **Phase 5**: 4 months
- **Phases 6-14**: 12 months
- **Phases 15-17**: 3 months
- **Total**: ~33 months (~2.75 years)

### Conservative Estimate
- **Total**: 48-60 months (4-5 years)

---

**Last Updated**: 2025-11-17
**Status**: Migration Planning Complete - Ready to Begin Implementation
