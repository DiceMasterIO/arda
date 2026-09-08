use super::annual_transfers::*;
use super::routing::{CellIndex, Extent};
use super::saddles::{Node, Saddle};
use arda_core::hydrology::{BasinId, Litres};
use arda_core::HeightMm;

fn limits() -> TransferLimits {
    TransferLimits {
        leaves: 10,
        ram_bytes: 1_000_000,
    }
}
fn wet(id: u64, net: i128, lake: u64, level: i32) -> LeafFlow {
    LeafFlow {
        leaf: BasinId(id),
        net_litres: net,
        lake: Some(BasinId(lake)),
        surface: Some(HeightMm::new(level)),
    }
}
fn dry(id: u64, net: i128) -> LeafFlow {
    LeafFlow {
        leaf: BasinId(id),
        net_litres: net,
        lake: None,
        surface: None,
    }
}
fn tree() -> (Extent, Vec<Saddle>) {
    let e = Extent::new(2, 1).unwrap();
    let a = CellIndex::new(0, e).unwrap();
    let b = CellIndex::new(1, e).unwrap();
    (
        e,
        vec![
            Saddle {
                left: Node::Closed(a),
                right: Node::Closed(b),
                sill_mm: 10,
                from: a,
                to: Some(b),
            },
            Saddle {
                left: Node::Closed(b),
                right: Node::Exterior,
                sill_mm: 10,
                from: b,
                to: None,
            },
        ],
    )
}

#[test]
fn upstream_supply_pays_receiver_loss_before_external_flow() {
    let (extent, edges) = tree();
    let result = normalize_transfers(
        extent,
        &[wet(0, 100, 100, 10), wet(1, -40, 101, 10)],
        &edges,
        limits(),
    )
    .unwrap();
    assert_eq!(result.exterior_outflow, Litres(60));
    assert_eq!(result.transfers.len(), 2);
    assert_eq!(result.transfers[0].annual_volume, Litres(100));
    assert_eq!(result.transfers[0].to_leaf, Some(BasinId(1)));
    assert_eq!(result.transfers[1].annual_volume, Litres(60));
    assert_eq!(result.transfers[1].to, None);
}

#[test]
fn dry_receiver_can_absorb_all_arriving_water() {
    let (extent, edges) = tree();
    let result = normalize_transfers(
        extent,
        &[wet(0, 40, 100, 10), dry(1, -40)],
        &edges,
        limits(),
    )
    .unwrap();
    assert_eq!(result.exterior_outflow, Litres(0));
    assert_eq!(result.transfers.len(), 1);
    assert_eq!(result.transfers[0].annual_volume, Litres(40));
}

#[test]
fn a_deficit_reverses_the_actual_witness_without_importing_ocean_water() {
    let (extent, edges) = tree();
    let result = normalize_transfers(
        extent,
        &[dry(0, -40), wet(1, 100, 101, 10)],
        &edges,
        limits(),
    )
    .unwrap();
    assert_eq!(result.exterior_outflow, Litres(60));
    let to_dry = result
        .transfers
        .iter()
        .find(|t| t.to_leaf == Some(BasinId(0)))
        .unwrap();
    assert_eq!((to_dry.from.x, to_dry.to.unwrap().x), (1, 0));
    assert_eq!(to_dry.annual_volume, Litres(40));
}

#[test]
fn connected_lake_internal_flows_are_not_exported_twice() {
    let (extent, mut edges) = tree();
    edges[1].sill_mm = 11;
    let result = normalize_transfers(
        extent,
        &[wet(0, 100, 42, 11), wet(1, -40, 42, 11)],
        &edges,
        limits(),
    )
    .unwrap();
    assert_eq!(result.transfers.len(), 1);
    assert_eq!(result.transfers[0].annual_volume, Litres(60));
    assert_eq!(result.exterior_outflow, Litres(60));
}

#[test]
fn equal_sill_surfaces_do_not_invent_positive_depth_connectivity() {
    let (extent, edges) = tree();
    let result = normalize_transfers(
        extent,
        &[wet(0, 100, 42, 10), wet(1, -40, 42, 10)],
        &edges,
        limits(),
    );
    assert_eq!(
        result,
        Err(TransferError::Invalid("disconnected shared lake identity"))
    );
}

#[test]
fn spill_requires_source_support_and_cannot_draw_from_exterior() {
    let (extent, edges) = tree();
    assert_eq!(
        normalize_transfers(extent, &[wet(0, 40, 100, 9), dry(1, -40)], &edges, limits()),
        Err(TransferError::Invalid("unsupported source spill"))
    );
    assert_eq!(
        normalize_transfers(extent, &[dry(0, -40), wet(1, 0, 101, 10)], &edges, limits()),
        Err(TransferError::Invalid("unfunded exterior import"))
    );
}

#[test]
fn edge_order_does_not_change_persistable_transfers() {
    let (extent, mut edges) = tree();
    let leaves = [wet(0, 100, 100, 10), wet(1, -40, 101, 10)];
    let forward = normalize_transfers(extent, &leaves, &edges, limits()).unwrap();
    edges.reverse();
    assert_eq!(
        forward,
        normalize_transfers(extent, &leaves, &edges, limits()).unwrap()
    );
}

#[test]
fn rejects_duplicate_edges_wrong_leaf_ids_and_unadmitted_buffers() {
    let (extent, mut edges) = tree();
    let leaves = [wet(0, 100, 100, 10), wet(1, -40, 101, 10)];
    edges[0] = edges[1];
    assert_eq!(
        normalize_transfers(extent, &leaves, &edges, limits()),
        Err(TransferError::Invalid("cycle or duplicate edge"))
    );
    let (extent, edges) = tree();
    assert!(matches!(
        normalize_transfers(extent, &[wet(2, 0, 100, 10), dry(3, 0)], &edges, limits()),
        Err(TransferError::Invalid(_))
    ));
    assert_eq!(
        normalize_transfers(
            extent,
            &leaves,
            &edges,
            TransferLimits {
                leaves: 10,
                ram_bytes: 0
            }
        ),
        Err(TransferError::Limit)
    );
}

#[test]
fn checks_signed_capacity_and_handles_an_empty_basin_set() {
    let (extent, edges) = tree();
    assert_eq!(
        normalize_transfers(
            extent,
            &[wet(0, i128::MAX, 100, 10), wet(1, 1, 101, 10)],
            &edges,
            limits()
        ),
        Err(TransferError::Overflow)
    );
    let empty = normalize_transfers(extent, &[], &[], limits()).unwrap();
    assert!(empty.transfers.is_empty());
    assert_eq!(empty.exterior_outflow, Litres(0));
}

#[test]
fn external_outflow_cannot_publish_a_lake_above_its_spill() {
    let (extent, edges) = tree();
    assert_eq!(
        normalize_transfers(
            extent,
            &[wet(0, 100, 42, 11), wet(1, -40, 42, 11)],
            &edges,
            limits()
        ),
        Err(TransferError::Invalid("unsupported source spill"))
    );
}

#[test]
fn zero_transfer_still_validates_shared_identity_and_surface() {
    let (extent, edges) = tree();
    for leaves in [
        [wet(0, 0, 42, 10), wet(1, 0, 42, 10)],
        [wet(0, 0, 42, 11), wet(1, 0, 42, 12)],
    ] {
        assert_eq!(
            normalize_transfers(extent, &leaves, &edges, limits()),
            Err(TransferError::Invalid("disconnected shared lake identity"))
        );
    }
}

#[test]
fn different_lake_ids_cannot_both_submerge_their_shared_sill() {
    let (extent, edges) = tree();
    for net in [0, 40] {
        assert_eq!(
            normalize_transfers(
                extent,
                &[wet(0, net, 42, 11), wet(1, -net, 43, 11)],
                &edges,
                limits()
            ),
            Err(TransferError::Invalid(
                "different lake identities submerge shared sill"
            ))
        );
    }
}
