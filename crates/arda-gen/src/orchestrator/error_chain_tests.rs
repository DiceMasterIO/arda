//! Pipeline diagnostics retain stage context and the original typed failure.
#![allow(clippy::unwrap_used)]

use super::{annual_source, flow_disk, prepared_files, routing_disk, shared_solve, GenError};
use crate::hydrology::{
    annual_records, annual_topology, area_output, extraction, final_index, fine_flow,
    flow_adjacency, flow_metrics, handoff, hierarchy, hierarchy_disk, local_rivers, mst, ocean,
    routing, saddles, witness_binding,
};
use std::error::Error;
use std::io;

fn messages(error: &dyn Error) -> Vec<String> {
    let mut messages = vec![error.to_string()];
    let mut next = error.source();
    while let Some(cause) = next {
        assert!(messages.len() < 16, "error source chain must terminate");
        messages.push(cause.to_string());
        next = cause.source();
    }
    messages
}

#[test]
fn metrics_limit_and_adjacency_corruption_reach_the_generator_error_chain() {
    let limit = GenError::Shared(shared_solve::SharedError::Metrics(
        flow_metrics::MetricsError::Store(flow_disk::FlowDiskError::Limit("I/O operations")),
    ));
    assert_eq!(
        messages(&limit),
        [
            "flow metrics failed",
            "flow metrics scratch failed",
            "fine-flow storage exceeded I/O operations",
        ]
    );
    let backend = limit.source().unwrap().source().unwrap();
    assert!(matches!(
        backend.downcast_ref::<flow_disk::FlowDiskError>(),
        Some(flow_disk::FlowDiskError::Limit("I/O operations"))
    ));
    let corrupt = GenError::Shared(shared_solve::SharedError::Metrics(
        flow_metrics::MetricsError::Adjacency(flow_adjacency::AdjacencyError::Store(
            flow_disk::FlowDiskError::Invalid("record checksum"),
        )),
    ));
    assert_eq!(
        messages(&corrupt),
        [
            "flow metrics failed",
            "flow metrics adjacency failed",
            "flow adjacency record read failed",
            "invalid fine-flow storage: record checksum",
        ]
    );
}

#[test]
fn final_index_and_area_wrappers_preserve_the_original_io_error() {
    let io_failure = || routing_disk::DiskError::Io {
        path: "fixture-routing.bin".into(),
        source: io::Error::new(io::ErrorKind::PermissionDenied, "fixture permissions"),
    };
    let errors = [
        GenError::Index(final_index::IndexError::Extract(
            extraction::ExtractError::Adjacency(flow_adjacency::AdjacencyError::Routing(
                io_failure(),
            )),
        )),
        GenError::Area(area_output::AreaError::Extract(
            extraction::ExtractError::Adjacency(flow_adjacency::AdjacencyError::Routing(
                io_failure(),
            )),
        )),
    ];
    for error in errors {
        assert_eq!(messages(&error).last().unwrap(), "fixture permissions");
        let mut cause: &dyn Error = &error;
        while let Some(next) = cause.source() {
            cause = next;
        }
        assert_eq!(
            cause.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::PermissionDenied
        );
    }
}

#[test]
fn concrete_shared_pipeline_wrappers_retain_limit_or_input_causes() {
    use shared_solve::SharedError;
    let errors = [
        SharedError::RoutingCreate(routing_disk::CreateError::Input(
            prepared_files::PreparedError::Invalid("fixture input"),
        )),
        SharedError::Ocean(ocean::OceanError::Storage(routing_disk::DiskError::Limit(
            "fixture input",
        ))),
        SharedError::Route(routing::RoutingError::Storage(
            routing_disk::DiskError::Limit("fixture input"),
        )),
        SharedError::Mst(mst::MstError::Routing(routing_disk::DiskError::Limit(
            "fixture input",
        ))),
        SharedError::Binding(witness_binding::BindingError::Source(
            routing_disk::DiskError::Limit("fixture input"),
        )),
        SharedError::Hierarchy(hierarchy::BuildError::Backend(
            handoff::HandoffError::Storage(hierarchy_disk::DiskError::Limit("fixture input")),
        )),
        SharedError::Topology(annual_topology::TopologyError::Input(
            hierarchy_disk::DiskError::Limit("fixture input"),
        )),
        SharedError::Records(annual_records::RecordError::Source(
            routing_disk::DiskError::Limit("fixture input"),
        )),
        SharedError::FlowCreate(flow_disk::CreateError::Storage(
            flow_disk::FlowDiskError::Limit("fixture input"),
        )),
        SharedError::Flow(fine_flow::FlowError::Store(
            flow_disk::FlowDiskError::Limit("fixture input"),
        )),
    ];
    for stage in errors {
        let error = GenError::Shared(stage);
        let chain = messages(&error);
        assert!(chain.len() >= 2, "original cause hidden: {chain:?}");
        assert!(chain.last().unwrap().ends_with("fixture input"));
    }
}

#[test]
fn generic_callbacks_and_nested_stage_wrappers_retain_typed_sources() {
    type E = io::Error;
    fn input() -> E {
        io::Error::other("fixture callback")
    }
    fn boxed(error: impl Error + 'static) -> Box<dyn Error> {
        Box::new(error)
    }
    let wrappers = [
        boxed(annual_records::RecordError::Source(input())),
        boxed(annual_topology::TopologyError::Binding(
            witness_binding::BindingError::Source(input()),
        )),
        boxed(area_output::AreaError::<E, E>::Routing(input())),
        boxed(area_output::AreaError::<E, E>::Flow(input())),
        boxed(extraction::ExtractError::<E, E>::Routing(input())),
        boxed(extraction::ExtractError::<E, E>::Flow(input())),
        boxed(final_index::IndexError::<E, E>::Routing(input())),
        boxed(final_index::IndexError::<E, E>::Flow(input())),
        boxed(fine_flow::FlowError::<E, E, E>::Routing(input())),
        boxed(fine_flow::FlowError::<E, E, E>::Stream(input())),
        boxed(flow_metrics::MetricsError::<E, E>::Routing(input())),
        boxed(handoff::HandoffError::<E, E>::Binding(
            witness_binding::BindingError::Source(input()),
        )),
        boxed(local_rivers::LocalRiverError::Source(input())),
        boxed(saddles::ScanError::<E, E>::Storage(input())),
        boxed(saddles::ScanError::<E, E>::Output(input())),
        boxed(annual_source::CellSourceError::Routing(input())),
        boxed(flow_disk::CreateError::Input(input())),
    ];
    for wrapper in wrappers {
        assert_eq!(
            messages(wrapper.as_ref()).last().unwrap(),
            "fixture callback"
        );
        let mut cause: &dyn Error = wrapper.as_ref();
        while let Some(next) = cause.source() {
            cause = next;
        }
        assert!(cause.downcast_ref::<E>().is_some());
    }
}
