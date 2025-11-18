# Current Implementation

## fe-common (Foundation) - 4 tests ✓
- DorisError (13 variants), Result<T>
- Config (TOML, 20+ fields)
- Types: IDs (9), enums (TableType, KeysType, DataType, etc.)
- Utils: timestamps, size parsing/formatting

## fe-catalog (Metadata) - 102 tests ✓
**Files**: catalog.rs, database.rs, table.rs, column.rs, partition.rs, index.rs, replica.rs

- **Catalog**: Root manager (create/drop DB/table)
- **Database**: Quotas (-1=unlimited), properties, table map
- **OlapTable**: columns, partitions, indexes, KeysType (AGG/DUP/UNIQUE)
- **Column**: types, nullable, agg_type, defaults, positions
- **Partition**: versions, tablets, temp flag, data size
- **Index**: base/rollup/bitmap/inverted types
- **Replica**: state machine, version tracking, health

**Tests**:
- catalog_tests (7), column_tests (6), partition_tests (5), replica_tests (4)
- database_extended (6), column_extended (11), table_tests (9)
- edge_case_tests (15), validation_tests (15)
- type_tests (15), serialization_tests (13)

## fe-analysis (SQL Parser) - 11 tests ✓
**Files**: parser.rs, ast.rs, analyzer.rs

- **Parser**: Uses sqlparser-rs (0.49)
- **DDL**: CREATE/DROP TABLE/DATABASE
- **DML**: SELECT/INSERT/UPDATE/DELETE
- **TPC-H**: Parses lineitem table, Q1 query
- **Analyzer**: Duplicate column detection

**Tests**:
- parser_tests (9): Simple SELECT, CREATE TABLE, TPC-H lineitem, TPC-H Q1, INSERT/UPDATE/DELETE
- analyzer_tests (2): Valid CREATE TABLE, duplicate columns

## fe-qe (Query Executor) - 28 tests ✓
**Files**: executor.rs, result.rs, select_tests.rs, comparison_tests.rs

- **Executor**: Executes parsed SQL statements
- **DDL**: CREATE TABLE (TPC-H lineitem ✓), DROP TABLE/DATABASE, USE
- **SELECT**: Column projection, aliases, aggregations (schema-only, no data)
- **Type Parsing**: INT, VARCHAR, DECIMAL, CHAR, DATE, DATETIME
- **Results**: QueryResult, ResultSet, Row, Value (11 variants)
- **TPC-H Q1**: Full query execution with 10 column result set
- **Comparison Tests**: Verifies Rust FE matches Java FE behavior (22 tests)

**Tests**:
- executor_tests (4): CREATE TABLE, TPC-H lineitem (16 cols), data types, DROP TABLE
- select_tests (2): Simple SELECT, TPC-H Q1 (10 columns with aliases)
- comparison_tests (22): SQL parsing (8), catalog ops (6), query exec (2), errors (4), types (4)

## fe-planner (Query Planner) - 9 tests ✓
**Files**: planner.rs, thrift_plan.rs

- **QueryPlanner**: Converts AST to execution plans
- **Thrift Structures**: TPlanNode, TPlanFragment, TOlapScanNode
- **Plan Types**: OLAP scan, aggregation, sort (35 node types)
- **Serialization**: JSON output for comparison with Java FE

**Tests**:
- thrift_plan_tests (5): Plan node serialization, fragment serialization
- planner_tests (4): Lineitem scan, JSON output, SELECT statement planning

## fe-mysql-protocol (MySQL Protocol) - 28 tests ✓
**Files**: constants.rs, packet.rs, handshake.rs, resultset.rs, server.rs

- **Packet**: MySQL wire protocol framing (header + payload)
- **Handshake**: Initial handshake, auth response parsing
- **Auth**: mysql_native_password verification
- **ResultSet**: Column definitions, row data, EOF/OK packets
- **Type Conversion**: Doris → MySQL type mapping
- **Server**: TCP listener, connection handler, command loop (COM_QUERY, COM_INIT_DB, COM_PING, COM_QUIT)
- **Integration**: Parser → Executor → Result encoding pipeline
- **System Queries**: SELECT @@, SET, SHOW (client compatibility)
- **TPC-H Q1**: Full end-to-end execution via MySQL CLI

**Tests**:
- packet_tests (6): Header, packet I/O, OK/ERR/EOF, length-encoded values
- handshake_tests (5): Initial handshake, auth data, password verification
- resultset_tests (7): Column defs, rows, result sets, type conversion
- server_tests (3): Server creation, DDL execution, parse error handling
- integration_tests (7): Connection, CREATE TABLE, database switching, TPC-H lineitem, error handling, TPC-H Q1

## fe-backend-client (C++ BE Client) - 6 tests ✓
**Files**: lib.rs, mock.rs

- **BackendClient**: gRPC client for C++ BE communication (stub)
- **MockBackend**: Testing without protoc/real BE
- **RPCs**: exec_plan_fragment, fetch_data (TODO: implement with gRPC)
- **Protocol**: gRPC/Protobuf (PBackendService from internal_service.proto)
- **Port**: 9060 (default BE gRPC port)

**Tests**:
- client_tests (1): Client creation
- mock_tests (5): Backend creation, exec fragment, fetch data (empty/with results), TPC-H Q1 mock

**Next**: Install protoc → generate bindings → implement real gRPC calls

## fe-main (Entry) - 0 tests
CLI, config, logging, signals, MySQL server startup

## Design
- DashMap #[serde(skip)]
- Arc<RwLock<T>> for shared state
- &Column refs vs Arc<Column>
- Thread-safe by default
