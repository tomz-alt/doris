// Minimal C++ helper to inspect PBlock protobufs produced by the Rust FE.
// This lives under rust-fe/tools and does not modify the C++ BE tree.
//
// Build instructions (see README.md for details):
//   protoc --cpp_out=. ../../proto/data.proto ../../proto/types.proto
//   g++ -std=c++17 -O2 -I. pblock_dump.cc data.pb.cc types.pb.cc -lprotobuf -o pblock_dump

#include <fstream>
#include <iostream>
#include <string>

#include "data.pb.h"

int main(int argc, char** argv) {
    if (argc < 2) {
        std::cerr << "Usage: pblock_dump <pblock.bin>\n";
        return 1;
    }

    const std::string path = argv[1];
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        std::cerr << "Failed to open file: " << path << "\n";
        return 1;
    }

    doris::PBlock block;
    if (!block.ParseFromIstream(&in)) {
        std::cerr << "Failed to parse PBlock from: " << path << "\n";
        return 1;
    }

    std::cout << "PBlock from " << path << ":\n";
    std::cout << "  column_metas: " << block.column_metas_size() << "\n";
    std::cout << "  column_values size: " << block.column_values().size() << " bytes\n";
    if (block.has_be_exec_version()) {
        std::cout << "  be_exec_version: " << block.be_exec_version() << "\n";
    } else {
        std::cout << "  be_exec_version: <not set>\n";
    }

    for (int i = 0; i < block.column_metas_size(); ++i) {
        const auto& meta = block.column_metas(i);
        std::cout << "  col[" << i << "]: name=" << meta.name()
                  << " type_id=" << meta.type() << " nullable=" << meta.is_nullable()
                  << "\n";
    }

    return 0;
}

