-- TPC-H SF100 Table Definitions for Doris
-- Matches official TPC-H specification v2.18.0
-- Optimized for 100GB scale factor with partitioning and distribution

-- Customer table (SF100: 15M rows)
CREATE TABLE IF NOT EXISTS customer (
    c_custkey     INT NOT NULL,
    c_name        VARCHAR(25) NOT NULL,
    c_address     VARCHAR(40) NOT NULL,
    c_nationkey   INT NOT NULL,
    c_phone       CHAR(15) NOT NULL,
    c_acctbal     DECIMAL(15,2) NOT NULL,
    c_mktsegment  CHAR(10) NOT NULL,
    c_comment     VARCHAR(117) NOT NULL
)
DUPLICATE KEY(c_custkey)
DISTRIBUTED BY HASH(c_custkey) BUCKETS 32
PROPERTIES (
    "replication_num" = "3",
    "storage_format" = "V2"
);

-- Lineitem table (SF100: 600M rows - largest table)
-- Partitioned by ship date for partition pruning optimization
CREATE TABLE IF NOT EXISTS lineitem (
    l_orderkey      INT NOT NULL,
    l_partkey       INT NOT NULL,
    l_suppkey       INT NOT NULL,
    l_linenumber    INT NOT NULL,
    l_quantity      DECIMAL(15,2) NOT NULL,
    l_extendedprice DECIMAL(15,2) NOT NULL,
    l_discount      DECIMAL(15,2) NOT NULL,
    l_tax           DECIMAL(15,2) NOT NULL,
    l_returnflag    CHAR(1) NOT NULL,
    l_linestatus    CHAR(1) NOT NULL,
    l_shipdate      DATE NOT NULL,
    l_commitdate    DATE NOT NULL,
    l_receiptdate   DATE NOT NULL,
    l_shipinstruct  CHAR(25) NOT NULL,
    l_shipmode      CHAR(10) NOT NULL,
    l_comment       VARCHAR(44) NOT NULL
)
DUPLICATE KEY(l_orderkey, l_partkey, l_suppkey, l_linenumber)
PARTITION BY RANGE(l_shipdate) (
    PARTITION p1992 VALUES LESS THAN ("1993-01-01"),
    PARTITION p1993 VALUES LESS THAN ("1994-01-01"),
    PARTITION p1994 VALUES LESS THAN ("1995-01-01"),
    PARTITION p1995 VALUES LESS THAN ("1996-01-01"),
    PARTITION p1996 VALUES LESS THAN ("1997-01-01"),
    PARTITION p1997 VALUES LESS THAN ("1998-01-01"),
    PARTITION p1998 VALUES LESS THAN ("1999-01-01"),
    PARTITION pmax VALUES LESS THAN MAXVALUE
)
DISTRIBUTED BY HASH(l_orderkey) BUCKETS 64
PROPERTIES (
    "replication_num" = "3",
    "storage_format" = "V2",
    "compression" = "LZ4"
);

-- Nation table (25 rows - reference data)
CREATE TABLE IF NOT EXISTS nation (
    n_nationkey INT NOT NULL,
    n_name      CHAR(25) NOT NULL,
    n_regionkey INT NOT NULL,
    n_comment   VARCHAR(152) NOT NULL
)
DUPLICATE KEY(n_nationkey)
DISTRIBUTED BY HASH(n_nationkey) BUCKETS 1
PROPERTIES (
    "replication_num" = "3"
);

-- Orders table (SF100: 150M rows)
-- Partitioned by order date for partition pruning optimization
CREATE TABLE IF NOT EXISTS orders (
    o_orderkey      INT NOT NULL,
    o_custkey       INT NOT NULL,
    o_orderstatus   CHAR(1) NOT NULL,
    o_totalprice    DECIMAL(15,2) NOT NULL,
    o_orderdate     DATE NOT NULL,
    o_orderpriority CHAR(15) NOT NULL,
    o_clerk         CHAR(15) NOT NULL,
    o_shippriority  INT NOT NULL,
    o_comment       VARCHAR(79) NOT NULL
)
DUPLICATE KEY(o_orderkey)
PARTITION BY RANGE(o_orderdate) (
    PARTITION p1992 VALUES LESS THAN ("1993-01-01"),
    PARTITION p1993 VALUES LESS THAN ("1994-01-01"),
    PARTITION p1994 VALUES LESS THAN ("1995-01-01"),
    PARTITION p1995 VALUES LESS THAN ("1996-01-01"),
    PARTITION p1996 VALUES LESS THAN ("1997-01-01"),
    PARTITION p1997 VALUES LESS THAN ("1998-01-01"),
    PARTITION p1998 VALUES LESS THAN ("1999-01-01"),
    PARTITION pmax VALUES LESS THAN MAXVALUE
)
DISTRIBUTED BY HASH(o_orderkey) BUCKETS 48
PROPERTIES (
    "replication_num" = "3",
    "storage_format" = "V2"
);

-- Part table (SF100: 20M rows)
CREATE TABLE IF NOT EXISTS part (
    p_partkey     INT NOT NULL,
    p_name        VARCHAR(55) NOT NULL,
    p_mfgr        CHAR(25) NOT NULL,
    p_brand       CHAR(10) NOT NULL,
    p_type        VARCHAR(25) NOT NULL,
    p_size        INT NOT NULL,
    p_container   CHAR(10) NOT NULL,
    p_retailprice DECIMAL(15,2) NOT NULL,
    p_comment     VARCHAR(23) NOT NULL
)
DUPLICATE KEY(p_partkey)
DISTRIBUTED BY HASH(p_partkey) BUCKETS 32
PROPERTIES (
    "replication_num" = "3",
    "storage_format" = "V2"
);

-- Partsupp table (SF100: 80M rows)
-- Colocated with part for bucket shuffle optimization
CREATE TABLE IF NOT EXISTS partsupp (
    ps_partkey    INT NOT NULL,
    ps_suppkey    INT NOT NULL,
    ps_availqty   INT NOT NULL,
    ps_supplycost DECIMAL(15,2) NOT NULL,
    ps_comment    VARCHAR(199) NOT NULL
)
DUPLICATE KEY(ps_partkey, ps_suppkey)
DISTRIBUTED BY HASH(ps_partkey) BUCKETS 32
PROPERTIES (
    "replication_num" = "3",
    "storage_format" = "V2",
    "colocate_with" = "part"
);

-- Region table (5 rows - reference data)
CREATE TABLE IF NOT EXISTS region (
    r_regionkey INT NOT NULL,
    r_name      CHAR(25) NOT NULL,
    r_comment   VARCHAR(152) NOT NULL
)
DUPLICATE KEY(r_regionkey)
DISTRIBUTED BY HASH(r_regionkey) BUCKETS 1
PROPERTIES (
    "replication_num" = "3"
);

-- Supplier table (SF100: 1M rows)
CREATE TABLE IF NOT EXISTS supplier (
    s_suppkey   INT NOT NULL,
    s_name      CHAR(25) NOT NULL,
    s_address   VARCHAR(40) NOT NULL,
    s_nationkey INT NOT NULL,
    s_phone     CHAR(15) NOT NULL,
    s_acctbal   DECIMAL(15,2) NOT NULL,
    s_comment   VARCHAR(101) NOT NULL
)
DUPLICATE KEY(s_suppkey)
DISTRIBUTED BY HASH(s_suppkey) BUCKETS 16
PROPERTIES (
    "replication_num" = "3",
    "storage_format" = "V2"
);

-- Verify tables created
SHOW TABLES;

-- Check row counts (after data load)
-- SELECT 'customer' as tbl, COUNT(*) FROM customer
-- UNION ALL SELECT 'lineitem', COUNT(*) FROM lineitem
-- UNION ALL SELECT 'nation', COUNT(*) FROM nation
-- UNION ALL SELECT 'orders', COUNT(*) FROM orders
-- UNION ALL SELECT 'part', COUNT(*) FROM part
-- UNION ALL SELECT 'partsupp', COUNT(*) FROM partsupp
-- UNION ALL SELECT 'region', COUNT(*) FROM region
-- UNION ALL SELECT 'supplier', COUNT(*) FROM supplier;

-- Expected counts for SF100:
-- customer:    15,000,000
-- lineitem:   600,037,902
-- nation:              25
-- orders:     150,000,000
-- part:        20,000,000
-- partsupp:    80,000,000
-- region:               5
-- supplier:     1,000,000
