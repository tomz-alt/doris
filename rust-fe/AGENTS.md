# Agent Guidelines for Rust FE Work

This file captures high-level design principles that all agent work on this
Rust Frontend should follow. The goal is a production-ready, high-performance
FE that can fully replace the Java FE over time.

1. **Keep the core boundary clean.**
   - Treat the core FE as: `execute_sql` + Doris-aware parser + catalog +
     planner + BE RPC layer.
   - All wire protocols (MySQL, pgwire, HTTP, extensions) should call this
     core API rather than embedding parsing/planning logic.

2. **Use Java FE behavior as the specification.**
   - When in doubt about semantics or protocol details, treat the current
     Java FE as the reference implementation.
   - Prefer systematic differential tests against Java FE over ad-hoc
     manual checks.

3. **Hide low-level transport details behind clear interfaces.**
   - Design load/stream-load and fragment/execution interfaces so they do
     not depend on particular wire formats or RPC libraries.
   - This allows evolving BE protocols (e.g., SQL vs protobuf vs Arrow IPC)
     without large refactors in the core.

4. **Prioritize resource control and observability.**
   - Changes should respect and, when reasonable, improve: queue limits,
     concurrency controls, backpressure, and graceful error handling.
   - Expose useful metrics and logs around query planning, BE calls,
     streaming load, and protocol activity.

Any new code or refactors in this repository should be evaluated against
these principles. When implementing larger features, verify at logical
milestones (e.g., new core boundaries, new BE paths, new load modes) with
targeted tests and, where applicable, comparisons to Java FE behavior.

