//! Limits and typed errors of the metrics pass, on the same fixtures.
use super::*;

#[test]
fn known_limits_reject_before_publication_and_exact_work_cap_is_sufficient() {
    let mut f = bowl();
    let mut short = limits();
    short.lakes = 0;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit(_))
    ));
    let mut f = bowl();
    let mut short = limits();
    short.ram_bytes = flow_metrics::required_ram(1, 0).unwrap() - 1;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit(_))
    ));
    let mut f = bowl();
    let mut short = limits();
    short.lake_edges = 0;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit(_))
    ));
    let mut f = bowl();
    let mut short = limits();
    short.ram_bytes = flow_metrics::required_ram(1, 0).unwrap();
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit(_))
    ));
    let mut f = dry_hand();
    let work = f.run();
    let mut f = dry_hand();
    let mut short = limits();
    short.operations = work.operations - 1;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit("operations"))
    ));
    let mut f = dry_hand();
    short.operations = work.operations;
    assert_eq!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short).unwrap(),
        work
    );
    assert!(flow_metrics::required_ram(u64::MAX, 0).is_none());
    assert!(flow_metrics::required_ram(0, u64::MAX).is_none());
}

#[test]
fn bad_lake_membership_nonfresh_scratch_and_huge_scalar_are_typed_errors() {
    let mut f = bowl();
    f.lakes[0].submerged_cells -= 1;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, limits()),
        Err(MetricsError::Invalid("lake membership count"))
    ));
    let mut f = bowl();
    f.lakes.clear();
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, limits()),
        Err(MetricsError::Invalid("unknown wet lake"))
    ));
    let mut f = dry_hand();
    f.run();
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, limits()),
        Err(MetricsError::Invalid("metrics scratch is not fresh"))
    ));
    let mut f = dry_hand();
    let at = f.at(8);
    let mut row = f.row(8);
    row.net = 1_i128 << 65;
    f.flow.write(at, row).unwrap();
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, limits()),
        Err(MetricsError::Overflow)
    ));
}

#[test]
fn adjacency_rejects_unfinished_asymmetric_and_unequal_lake_state() {
    let mut f = fork();
    let at = f.at(10);
    let mut row = f.row(10);
    row.visited = false;
    f.flow.write(at, row).unwrap();
    assert!(matches!(
        flow_adjacency::incident(&mut f.routing, &mut f.flow, at),
        Err(AdjacencyError::Invalid("unfinished fine flow"))
    ));
    let mut f = fork();
    let at = f.at(10);
    let mut row = f.row(10);
    row.selected = 0;
    f.flow.write(at, row).unwrap();
    assert!(matches!(
        flow_adjacency::incident(&mut f.routing, &mut f.flow, at),
        Err(AdjacencyError::Invalid("asymmetric saddle"))
    ));
    let mut f = bowl();
    let at = f.at(12);
    let mut row = f.row(12);
    row.surface_mm += 1;
    f.flow.write(at, row).unwrap();
    assert!(matches!(
        flow_adjacency::incident(&mut f.routing, &mut f.flow, at),
        Err(AdjacencyError::Invalid("disconnected lake identity"))
    ));
}
