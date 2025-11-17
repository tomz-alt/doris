This directory contains an **optional** C++ helper for debugging and
cross-checking `PBlock` payloads produced by the Rust FE against Doris
BE expectations, while keeping all code under `rust-fe/` and **not**
modifying the C++ BE tree.

The intended workflow is:

1. Use the Rust FE to generate a `PBlock`:
   - The `BeLoadHelper::build_pblock_for_insert` function in
     `src/be/load.rs` builds a `PBlock` for a given `ParsedInsert`.
   - A small Rust binary (to be added) will write this `PBlock` as a
     protobuf-encoded file under `target/` for a chosen table.

2. Build this helper against generated C++ protobufs:
   - From `rust-fe/tools/pblock_dump`:
     - Generate `data.pb.h/cc` and `types.pb.h/cc` with:
       - `protoc --cpp_out=. ../../proto/data.proto ../../proto/types.proto`
     - Build `pblock_dump`:
       - `g++ -std=c++17 -O2 -I. pblock_dump.cc data.pb.cc types.pb.cc -lprotobuf -o pblock_dump`

3. Run the helper:
   - `./pblock_dump /path/to/pblock.bin`
   - It will parse the `PBlock` and print basic shape information
     (column count, column_metas, column_values size).

In a later step this helper can be extended to:

  - Include the BE `Block` implementation by adding appropriate
    include paths for `../be/src/vec/core/block.h` and linking against
    a built BE, then:
    - Call `Block::deserialize` on the parsed `PBlock`.
    - Print decoded row values for end-to-end verification.

At the moment this helper is *not* wired into `cargo test` and does
not affect the BE codebase. It is purely a debugging / validation
tool, designed to keep all cross-language experimentation inside the
`rust-fe` tree in line with `AGENTS.md` (core boundary stays at
`execute_sql`; protocol details and binary layouts are isolated in
the BE-facing layer).

