// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

package org.apache.doris.thrift;

import org.apache.doris.thrift.TPipelineFragmentParams;
import org.apache.doris.thrift.TPipelineFragmentParamsList;
import org.apache.doris.thrift.TPlanFragment;
import org.apache.doris.thrift.TPlan;
import org.apache.doris.thrift.TDescriptorTable;
import org.apache.doris.thrift.TUniqueId;
import org.apache.doris.thrift.TPipelineInstanceParams;
import org.apache.doris.thrift.TQueryGlobals;
import org.apache.doris.thrift.TQueryOptions;
import org.apache.thrift.TSerializer;
import org.apache.thrift.protocol.TCompactProtocol;

import java.io.FileOutputStream;
import java.util.ArrayList;
import java.util.HashMap;

/**
 * Test program to generate Thrift serialization for comparison with Rust FE.
 * This creates the same minimal structure as the Rust hex_dump example.
 */
public class ThriftSerializationTest {
    public static void main(String[] args) throws Exception {
        System.out.println("Creating minimal TPipelineFragmentParamsList for debugging\n");

        // Create unique ID - same as Rust: hi=12345, lo=67890
        TUniqueId uniqueId = new TUniqueId();
        uniqueId.setHi(12345);
        uniqueId.setLo(67890);

        // Create minimal descriptor table
        TDescriptorTable descTbl = new TDescriptorTable();
        descTbl.setTupleDescriptors(new ArrayList<>());
        // slot_descriptors and table_descriptors are optional, leave unset

        // Create minimal plan fragment
        TPlanFragment fragment = new TPlanFragment();
        TPlan plan = new TPlan();
        plan.setNodes(new ArrayList<>());  // Empty nodes list
        fragment.setPlan(plan);

        // Create minimal local params
        TPipelineInstanceParams localParam = new TPipelineInstanceParams();
        localParam.setFragmentInstanceId(uniqueId);
        localParam.setBackendNum(0);
        localParam.setSenderId(0);
        localParam.setPerNodeScanRanges(new HashMap<>());

        ArrayList<TPipelineInstanceParams> localParams = new ArrayList<>();
        localParams.add(localParam);

        // Create minimal TQueryGlobals
        TQueryGlobals queryGlobals = new TQueryGlobals();
        queryGlobals.setNowString("2025-11-19 06:03:30");  // Match Rust's minimal()
        queryGlobals.setTimestampMs(1732000000000L);       // Approximate match
        queryGlobals.setTimeZone("UTC");

        // Create minimal TQueryOptions
        TQueryOptions queryOptions = new TQueryOptions();
        queryOptions.setBatchSize(4096);
        queryOptions.setMemLimit(2147483648L);  // 2GB
        queryOptions.setQueryTimeout(3600);     // 1 hour

        // Create TPipelineFragmentParams matching Rust structure
        TPipelineFragmentParams params = new TPipelineFragmentParams();
        params.setProtocolVersion(org.apache.doris.thrift.PaloInternalServiceVersion.V1);
        params.setQueryId(uniqueId);
        params.setFragmentId(0);
        params.setPerExchNumSenders(new HashMap<>());
        params.setDescTbl(descTbl);
        params.setDestinations(new ArrayList<>());  // Empty but REQUIRED!
        params.setFragment(fragment);
        params.setLocalParams(localParams);
        // coord is optional, leave unset
        params.setNumSenders(1);
        params.setQueryGlobals(queryGlobals);
        params.setQueryOptions(queryOptions);
        params.setFragmentNumOnHost(1);
        params.setBackendId(10001L);
        params.setTotalInstances(1);
        params.setIsNereids(true);

        // Create params list
        TPipelineFragmentParamsList paramsList = new TPipelineFragmentParamsList();
        ArrayList<TPipelineFragmentParams> list = new ArrayList<>();
        list.add(params);
        paramsList.setParamsList(list);

        // Serialize using TCompactProtocol (same as BE expects)
        TSerializer serializer = new TSerializer(new TCompactProtocol.Factory());
        byte[] bytes = serializer.serialize(paramsList);

        System.out.println("✓ Successfully serialized " + bytes.length + " bytes\n");
        System.out.println("Hex dump:");
        printHexDump(bytes);

        System.out.println("\n\nFirst 100 bytes (for quick inspection):");
        for (int i = 0; i < Math.min(100, bytes.length); i++) {
            if (i % 16 == 0) {
                System.out.printf("\n%04x: ", i);
            }
            System.out.printf("%02x ", bytes[i] & 0xFF);
        }
        System.out.println("\n");

        // Save to file for comparison
        try (FileOutputStream fos = new FileOutputStream("/tmp/java_thrift_dump.bin")) {
            fos.write(bytes);
        }
        System.out.println("Binary dump saved to /tmp/java_thrift_dump.bin");

        // Compare with Rust output if it exists
        try {
            java.io.File rustFile = new java.io.File("/tmp/rust_thrift_dump.bin");
            if (rustFile.exists()) {
                java.nio.file.Path rustPath = rustFile.toPath();
                byte[] rustBytes = java.nio.file.Files.readAllBytes(rustPath);
                System.out.println("\n========== COMPARISON ==========");
                System.out.println("Java bytes:  " + bytes.length);
                System.out.println("Rust bytes:  " + rustBytes.length);

                if (bytes.length == rustBytes.length) {
                    boolean identical = true;
                    int firstDiff = -1;
                    for (int i = 0; i < bytes.length; i++) {
                        if (bytes[i] != rustBytes[i]) {
                            if (identical) {
                                firstDiff = i;
                                identical = false;
                            }
                        }
                    }
                    if (identical) {
                        System.out.println("✓ IDENTICAL! Serialization matches perfectly.");
                    } else {
                        System.out.println("✗ DIFFERENT at byte " + firstDiff);
                        System.out.println("\nBytes around first difference:");
                        int start = Math.max(0, firstDiff - 16);
                        int end = Math.min(bytes.length, firstDiff + 16);
                        System.out.println("Java:");
                        for (int i = start; i < end; i++) {
                            if (i == firstDiff) System.out.print("[");
                            System.out.printf("%02x ", bytes[i] & 0xFF);
                            if (i == firstDiff) System.out.print("]");
                        }
                        System.out.println("\nRust:");
                        for (int i = start; i < end; i++) {
                            if (i == firstDiff) System.out.print("[");
                            System.out.printf("%02x ", rustBytes[i] & 0xFF);
                            if (i == firstDiff) System.out.print("]");
                        }
                        System.out.println();
                    }
                } else {
                    System.out.println("✗ DIFFERENT LENGTHS");
                }
            }
        } catch (Exception e) {
            System.out.println("(Rust dump not found for comparison)");
        }
    }

    private static void printHexDump(byte[] bytes) {
        for (int i = 0; i < bytes.length; i += 16) {
            System.out.printf("%08x  ", i);

            // Print hex
            for (int j = 0; j < 16; j++) {
                if (i + j < bytes.length) {
                    System.out.printf("%02x ", bytes[i + j] & 0xFF);
                } else {
                    System.out.print("   ");
                }
                if (j == 7) System.out.print(" ");
            }

            // Print ASCII
            System.out.print(" |");
            for (int j = 0; j < 16 && i + j < bytes.length; j++) {
                byte b = bytes[i + j];
                if (b >= 32 && b < 127) {
                    System.out.print((char) b);
                } else {
                    System.out.print(".");
                }
            }
            System.out.println("|");
        }
    }
}
