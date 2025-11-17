#![allow(clippy::all)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_extern_crates)]

// The Apache Thrift Rust generator emits one Rust file per IDL, with
// module-level `use crate::...` references between them. We expose
// each generated file as its own Rust module so callers can use
// `doris_thrift::palo_internal_service::TPipelineFragmentParamsList`,
// `doris_thrift::types::TUniqueId`, etc.

pub mod data {
    include!(concat!(env!("OUT_DIR"), "/data.rs"));
}

pub mod data_sinks {
    include!(concat!(env!("OUT_DIR"), "/data_sinks.rs"));
}

pub mod external_table_schema {
    include!(concat!(env!("OUT_DIR"), "/external_table_schema.rs"));
}

pub mod descriptors {
    include!(concat!(env!("OUT_DIR"), "/descriptors.rs"));
}

pub mod exprs {
    include!(concat!(env!("OUT_DIR"), "/exprs.rs"));
}

pub mod plan_nodes {
    include!(concat!(env!("OUT_DIR"), "/plan_nodes.rs"));
}

pub mod planner {
    include!(concat!(env!("OUT_DIR"), "/planner.rs"));
}

pub mod query_cache {
    include!(concat!(env!("OUT_DIR"), "/query_cache.rs"));
}

pub mod runtime_profile {
    include!(concat!(env!("OUT_DIR"), "/runtime_profile.rs"));
}

pub mod metrics {
    include!(concat!(env!("OUT_DIR"), "/metrics.rs"));
}

pub mod status {
    include!(concat!(env!("OUT_DIR"), "/status.rs"));
}

pub mod types {
    include!(concat!(env!("OUT_DIR"), "/types.rs"));
}

pub mod partitions {
    include!(concat!(env!("OUT_DIR"), "/partitions.rs"));
}

pub mod opcodes {
    include!(concat!(env!("OUT_DIR"), "/opcodes.rs"));
}

pub mod palo_internal_service {
    include!(concat!(env!("OUT_DIR"), "/palo_internal_service.rs"));
}
