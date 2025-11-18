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

## fe-qe (Query Executor) - 4 tests ✓
**Files**: executor.rs, result.rs

- **Executor**: Executes parsed SQL statements
- **DDL**: CREATE TABLE (TPC-H lineitem ✓), DROP TABLE/DATABASE
- **Type Parsing**: INT, VARCHAR, DECIMAL, CHAR, DATE
- **Results**: QueryResult, ResultSet, Row, Value

**Tests**:
- executor_tests (4): CREATE TABLE, TPC-H lineitem (16 cols), data types, DROP TABLE

## fe-planner (Query Planner) - 9 tests ✓
**Files**: planner.rs, thrift_plan.rs

- **QueryPlanner**: Converts AST to execution plans
- **Thrift Structures**: TPlanNode, TPlanFragment, TOlapScanNode
- **Plan Types**: OLAP scan, aggregation, sort (35 node types)
- **Serialization**: JSON output for comparison with Java FE

**Tests**:
- thrift_plan_tests (5): Plan node serialization, fragment serialization
- planner_tests (4): Lineitem scan, JSON output, SELECT statement planning

## fe-mysql-protocol (MySQL Protocol) - 21 tests ✓
**Files**: constants.rs, packet.rs, handshake.rs, resultset.rs, server.rs

- **Packet**: MySQL wire protocol framing (header + payload)
- **Handshake**: Initial handshake, auth response parsing
- **Auth**: mysql_native_password verification
- **ResultSet**: Column definitions, row data, EOF/OK packets
- **Type Conversion**: Doris → MySQL type mapping
- **Server**: TCP listener, connection handler, command loop (COM_QUERY, COM_INIT_DB, COM_PING, COM_QUIT)
- **Integration**: Parser → Executor → Result encoding pipeline

**Tests**:
- packet_tests (6): Header, packet I/O, OK/ERR/EOF, length-encoded values
- handshake_tests (5): Initial handshake, auth data, password verification
- resultset_tests (7): Column defs, rows, result sets, type conversion
- server_tests (3): Server creation, DDL execution, parse error handling

## fe-main (Entry) - 0 tests
CLI, config, logging, signals

## Design
- DashMap #[serde(skip)]
- Arc<RwLock<T>> for shared state
- &Column refs vs Arc<Column>
- Thread-safe by default
