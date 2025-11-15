# TPC-DS Benchmark (Coming Soon)

This directory will contain TPC-DS benchmark infrastructure following the same pattern as TPC-H.

## Planned Structure

```
tpcds/
├── README.md                 # This file
├── generate_data.sh          # TPC-DS data generation script
├── queries/                  # TPC-DS query files
│   ├── q1.sql               # Query 1
│   ├── q2.sql               # Query 2
│   └── ...                  # Up to Q99
└── benchmark_tpcds.py        # Python benchmark runner (copy from TPC-H)
```

## TPC-DS Overview

TPC-DS is the decision support benchmark. It includes:
- **99 queries** (vs TPC-H's 22)
- **24 tables** (vs TPC-H's 8)
- More complex query patterns
- Testing modern OLAP scenarios

## Implementation Status

- [ ] Data generation script
- [ ] Query files (99 queries)
- [ ] Benchmark runner (same as TPC-H)
- [ ] HTML report generator

## Quick Start (When Ready)

```bash
# Generate SF1 data
./scripts/tpcds/generate_data.sh --scale 1

# Run benchmark
python3 scripts/benchmark_tpcds.py --scale 1 --rounds 3
```

## Resources

- [TPC-DS Specification](http://www.tpc.org/tpcds/)
- [TPC-DS Tools](https://www.tpc.org/tpc_documents_current_versions/current_specifications5.asp)
- Query templates available in TPC-DS specification

## Development

To implement TPC-DS:

1. **Download dsgen tool**:
   ```bash
   git clone https://github.com/gregrahn/tpcds-kit.git
   cd tpcds-kit/tools
   make
   ```

2. **Generate data**:
   ```bash
   ./dsdgen -scale 1 -dir /tmp/tpcds-data
   ```

3. **Create table DDL** based on TPC-DS specification

4. **Extract queries** from specification

5. **Copy benchmark_tpch.py** and adapt for TPC-DS

## Contributing

Help implement TPC-DS benchmark:
- Create table DDL
- Extract and format 99 queries
- Adapt data generation script from TPC-H template
- Test with SF1 data

---

For now, use TPC-H benchmark which is fully functional.
