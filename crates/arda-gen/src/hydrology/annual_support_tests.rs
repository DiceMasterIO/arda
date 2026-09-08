#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use arda_core::hydrology::{BasinId, Litres, ReceivingAccount, SpillConnection};
use arda_core::{GlobalCell, HeightMm};

fn band(owner: u64, bed: i32, cost: u128) -> AnnualBand {
    AnnualBand {
        owner: BasinId(owner),
        bed: HeightMm::new(bed),
        cells: 1,
        precipitation: Litres(0),
        effective_land_loss: Litres(0),
        open_water_evaporation: Litres(cost),
    }
}
#[allow(clippy::too_many_arguments)]
fn node(
    id: u64,
    parent: Option<u64>,
    floor: i32,
    birth: i32,
    sill: i32,
    source: u128,
    children: std::ops::Range<usize>,
    bands: std::ops::Range<usize>,
) -> AnnualNode {
    AnnualNode {
        id: BasinId(id),
        parent: parent.map(BasinId),
        floor: HeightMm::new(floor),
        birth: HeightMm::new(birth),
        spill: Some(SpillConnection {
            from: GlobalCell { x: 0, y: 0 },
            to: None,
            sill: HeightMm::new(sill),
            receiving: ReceivingAccount::DomainExport,
        }),
        destination: parent.is_none().then_some(AnnualDestination::DomainExport),
        children,
        bands,
        local_runoff: Litres(source),
        local_precipitation: Litres(source),
        local_land_loss: Litres(0),
    }
}
fn join(parent: u64, l: u64, r: u64, ll: u64, rr: u64, w: u64) -> AnnualJoin {
    AnnualJoin {
        parent: BasinId(parent),
        left_child: BasinId(l),
        right_child: BasinId(r),
        left_leaf: BasinId(ll),
        right_leaf: BasinId(rr),
        witness: w,
    }
}
fn limits() -> AnnualLimits {
    AnnualLimits {
        ram_bytes: 100_000_000,
        nodes: 10_000,
        bands: 10_000,
        events: 1_000_000,
    }
}
fn solve(
    nodes: &[AnnualNode],
    children: &[BasinId],
    bands: &[AnnualBand],
    joins: &[AnnualJoin],
) -> AnnualSolution {
    let out = solve_annual(
        AnnualInput {
            nodes,
            children,
            bands,
            joins,
        },
        limits(),
    )
    .unwrap();
    let b = out.balance;
    assert_eq!(
        b.land_precipitation.0 + b.lake_precipitation.0,
        b.land_loss.0
            + b.lake_evaporation.0
            + b.marginal_evaporation.0
            + b.sea_outflow.0
            + b.domain_outflow.0
    );
    assert_eq!(
        out.leaf_net.iter().map(|x| x.net_litres).sum::<i128>(),
        i128::try_from(b.sea_outflow.0 + b.domain_outflow.0).unwrap()
    );
    out
}
#[test]
fn dry_partial_exact_wet_and_zero_loss_are_distinct() {
    for (s, d, lakes, h, marginal, outflow) in [
        (0, 0, 0, 0, 0, 0),
        (0, 30, 0, 0, 0, 0),
        (20, 30, 0, 0, 20, 0),
        (30, 30, 1, 1, 0, 0),
        (80, 30, 1, 10, 0, 50),
        (80, 0, 1, 10, 0, 80),
    ] {
        let out = solve(
            &[node(1, None, 0, 0, 10, s, 0..0, 0..1)],
            &[],
            &[band(1, 0, d)],
            &[],
        );
        assert_eq!(out.lakes.len(), lakes);
        if lakes > 0 {
            assert_eq!(out.lakes[0].surface.raw(), h);
            assert_eq!(
                out.lakes[0].geometric_volume.0,
                u128::try_from(h).unwrap() * 10_000
            );
        }
        assert_eq!(out.balance.marginal_evaporation.0, marginal);
        assert_eq!(out.balance.domain_outflow.0, outflow);
    }
}
#[test]
fn direct_lake_rain_replaces_land_source_exactly() {
    let mut n = node(1, None, 0, 0, 10, 80, 0..0, 0..1);
    n.local_precipitation = Litres(100);
    n.local_land_loss = Litres(20);
    let mut b = band(1, 0, 50);
    b.precipitation = Litres(60);
    b.effective_land_loss = Litres(20);
    let out = solve(&[n], &[], &[b], &[]);
    assert_eq!(out.balance.land_precipitation.0, 40);
    assert_eq!(out.balance.land_loss.0, 0);
    assert_eq!(out.balance.lake_precipitation.0, 60);
    assert_eq!(out.balance.lake_evaporation.0, 50);
    assert_eq!(out.balance.domain_outflow.0, 50);
    assert_eq!(out.leaf_net[0].paid_wet_cost.0, 30);
}
#[test]
fn deep_pool_stops_on_broad_unsupported_shelf() {
    let mut shelf = band(1, 5, 100);
    shelf.cells = 100;
    let out = solve(
        &[node(1, None, 0, 0, 20, 40, 0..0, 0..2)],
        &[],
        &[band(1, 0, 10), shelf],
        &[],
    );
    assert_eq!(out.lakes[0].surface.raw(), 5);
    assert_eq!(out.lakes[0].submerged_cells, 1);
    assert_eq!(out.lakes[0].geometric_volume.0, 50_000);
    assert_eq!(out.balance.marginal_evaporation.0, 30);
}
fn two(
    source: u128,
) -> (
    Vec<AnnualNode>,
    Vec<BasinId>,
    Vec<AnnualBand>,
    Vec<AnnualJoin>,
) {
    (
        vec![
            node(1, Some(3), 0, 0, 10, source, 0..0, 0..1),
            node(2, Some(3), 0, 0, 10, 0, 0..0, 1..2),
            node(3, None, 0, 10, 20, 0, 0..2, 2..3),
        ],
        vec![BasinId(1), BasinId(2)],
        vec![band(1, 0, 10), band(2, 0, 10), band(1, 10, 20)],
        vec![join(3, 1, 2, 1, 2, 0)],
    )
}
#[test]
fn spill_funds_dry_receiver_before_parent_and_overflow() {
    for (source, expected_lakes, export) in
        [(15, 1, 0), (20, 2, 0), (30, 2, 0), (40, 1, 0), (80, 1, 40)]
    {
        let (n, c, b, j) = two(source);
        let out = solve(&n, &c, &b, &j);
        assert_eq!(out.lakes.len(), expected_lakes, "source {source}");
        assert_eq!(out.balance.domain_outflow.0, export);
        if source == 15 {
            assert_eq!(out.leaf_net[1].net_litres, -5);
            assert_eq!(out.leaf_net[1].lake, None);
        }
        if source == 30 {
            assert_eq!(out.lakes[0].surface.raw(), 10);
            assert_eq!(out.lakes[1].surface.raw(), 10);
            assert_eq!(out.balance.marginal_evaporation.0, 10);
        }
        if source == 40 {
            assert_eq!(out.lakes[0].basin, BasinId(3));
            assert_eq!(out.lakes[0].surface.raw(), 11);
        }
        if source == 80 {
            assert_eq!(out.leaf_net[0].net_litres, 50);
            assert_eq!(out.leaf_net[1].net_litres, -10);
        }
    }
}
#[test]
fn exact_zero_depth_join_does_not_merge_published_lakes() {
    let n = vec![
        node(1, Some(3), 0, 0, 10, 10, 0..0, 0..2),
        node(2, Some(3), 0, 0, 10, 10, 0..0, 2..4),
        node(3, None, 0, 10, 20, 0, 0..2, 4..5),
    ];
    let out = solve(
        &n,
        &[BasinId(1), BasinId(2)],
        &[
            band(1, 0, 0),
            band(1, 9, 10),
            band(2, 0, 0),
            band(2, 9, 10),
            band(1, 10, 20),
        ],
        &[join(3, 1, 2, 1, 2, 0)],
    );
    assert_eq!(
        out.lakes
            .iter()
            .map(|l| (l.basin.0, l.surface.raw()))
            .collect::<Vec<_>>(),
        vec![(1, 10), (2, 10)]
    );
}
#[test]
fn tied_three_child_tree_routes_past_full_subset_without_looping() {
    let n = vec![
        node(1, Some(4), 0, 0, 10, 100, 0..0, 0..1),
        node(2, Some(4), 0, 0, 10, 0, 0..0, 1..2),
        node(3, Some(4), 0, 0, 10, 0, 0..0, 2..3),
        node(4, None, 0, 10, 20, 0, 0..3, 3..4),
    ];
    let c = vec![BasinId(1), BasinId(2), BasinId(3)];
    let b = vec![
        band(1, 0, 10),
        band(2, 0, 20),
        band(3, 0, 30),
        band(2, 10, 10),
    ];
    let j = vec![join(4, 1, 2, 1, 2, 0), join(4, 2, 3, 2, 3, 1)];
    let out = solve(&n, &c, &b, &j);
    assert_eq!(out.lakes[0].basin, BasinId(4));
    assert_eq!(out.balance.domain_outflow.0, 30);
    let reversed = solve(&n, &[BasinId(3), BasinId(2), BasinId(1)], &b, &[j[1], j[0]]);
    assert_eq!(out.lakes, reversed.lakes);
    assert_eq!(out.leaf_net, reversed.leaf_net);
    assert_eq!(out.balance, reversed.balance);
}
#[test]
fn nested_join_retains_all_paid_child_costs() {
    let n = vec![
        node(1, Some(4), 0, 0, 10, 100, 0..0, 0..1),
        node(2, Some(4), 0, 0, 10, 0, 0..0, 1..2),
        node(3, Some(5), 0, 0, 20, 0, 0..0, 2..3),
        node(4, Some(5), 0, 10, 20, 0, 0..2, 3..4),
        node(5, None, 0, 20, 30, 0, 2..4, 4..5),
    ];
    let out = solve(
        &n,
        &[BasinId(1), BasinId(2), BasinId(4), BasinId(3)],
        &[
            band(1, 0, 10),
            band(2, 0, 10),
            band(3, 0, 10),
            band(1, 10, 10),
            band(2, 20, 10),
        ],
        &[join(4, 1, 2, 1, 2, 0), join(5, 4, 3, 2, 3, 1)],
    );
    assert_eq!(out.lakes[0].basin, BasinId(5));
    assert_eq!(out.lakes[0].submerged_cells, 5);
    assert_eq!(out.balance.domain_outflow.0, 50);
}
#[test]
fn partial_equal_height_cost_uses_owner_tied_largest_remainders() {
    let (mut n, c, _, j) = two(25);
    n[2].bands = 2..4;
    let b = vec![
        band(1, 0, 10),
        band(2, 0, 10),
        band(1, 10, 3),
        band(2, 10, 3),
    ];
    let out = solve(&n, &c, &b, &j);
    assert_eq!(out.leaf_net[0].marginal_cost.0, 3);
    assert_eq!(out.leaf_net[1].marginal_cost.0, 2);
    let mut reverse = b.clone();
    reverse.swap(2, 3);
    let r = solve(&n, &c, &reverse, &j);
    assert_eq!(out.leaf_net, r.leaf_net);
}
#[test]
fn independent_roots_route_to_real_terminal_and_then_sea() {
    let mut one = node(1, None, 0, 0, 10, 100, 0..0, 0..1);
    one.destination = Some(AnnualDestination::Basin(BasinId(2)));
    let mut two = node(2, None, -10, -10, 0, 0, 0..0, 1..2);
    two.destination = Some(AnnualDestination::Sea);
    let out = solve(&[one, two], &[], &[band(1, 0, 10), band(2, -10, 30)], &[]);
    assert_eq!(out.balance.sea_outflow.0, 60);
    assert_eq!(out.leaf_net[1].net_litres, -30);
    assert_eq!(out.lakes[1].surface.raw(), 0);
}
#[test]
fn malformed_topology_forcing_spans_and_limits_fail() {
    let (n, c, b, j) = two(80);
    let run = |nn: &[AnnualNode], cc: &[BasinId], bb: &[AnnualBand], jj: &[AnnualJoin], lim| {
        solve_annual(
            AnnualInput {
                nodes: nn,
                children: cc,
                bands: bb,
                joins: jj,
            },
            lim,
        )
    };
    let mut wrong = b.clone();
    wrong[0].effective_land_loss = Litres(11);
    assert!(matches!(
        run(&n, &c, &wrong, &j, limits()),
        Err(AnnualError::Invalid(_))
    ));
    let mut wrong = n.clone();
    wrong[0].bands = 0..0;
    assert!(matches!(
        run(&wrong, &c, &b, &j, limits()),
        Err(AnnualError::Invalid(_))
    ));
    assert!(matches!(
        run(&n, &c, &b, &[], limits()),
        Err(AnnualError::Invalid(_))
    ));
    let mut wrong = j.clone();
    wrong[0].right_leaf = BasinId(1);
    assert!(matches!(
        run(&n, &c, &b, &wrong, limits()),
        Err(AnnualError::Invalid(_))
    ));
    for lim in [
        AnnualLimits {
            ram_bytes: 1,
            ..limits()
        },
        AnnualLimits {
            events: 1,
            ..limits()
        },
        AnnualLimits {
            nodes: 1,
            ..limits()
        },
        AnnualLimits {
            bands: 1,
            ..limits()
        },
    ] {
        assert!(matches!(
            run(&n, &c, &b, &j, lim),
            Err(AnnualError::Limit(_))
        ));
    }
    let mut wrong = n.clone();
    wrong[0].local_precipitation = Litres(u128::MAX);
    wrong[0].local_runoff = Litres(u128::MAX);
    wrong[1].local_precipitation = Litres(1);
    wrong[1].local_runoff = Litres(1);
    assert_eq!(
        run(&wrong, &c, &b, &j, limits()),
        Err(AnnualError::Overflow)
    );
    assert_eq!(required_ram(u64::MAX, 0), Err(AnnualError::Overflow));
}
#[test]
fn root_cycle_and_duplicate_join_are_rejected() {
    let mut n1 = node(1, None, 0, 0, 10, 10, 0..0, 0..1);
    n1.destination = Some(AnnualDestination::Basin(BasinId(2)));
    let mut n2 = node(2, None, 0, 0, 10, 0, 0..0, 1..2);
    n2.destination = Some(AnnualDestination::Basin(BasinId(1)));
    assert!(matches!(
        solve_annual(
            AnnualInput {
                nodes: &[n1, n2],
                children: &[],
                bands: &[band(1, 0, 1), band(2, 0, 1)],
                joins: &[]
            },
            limits()
        ),
        Err(AnnualError::Invalid("root destination cycle"))
    ));
    let (n, c, b, j) = two(80);
    assert!(matches!(
        solve_annual(
            AnnualInput {
                nodes: &n,
                children: &c,
                bands: &b,
                joins: &[j[0], j[0]]
            },
            limits()
        ),
        Err(AnnualError::Invalid(_))
    ));
}
#[test]
fn zero_node_world_and_large_admitted_amounts_close() {
    let out = solve(&[], &[], &[], &[]);
    assert!(out.lakes.is_empty());
    let out = solve(
        &[node(
            1,
            None,
            i32::MIN,
            i32::MIN,
            i32::MAX,
            MAX_ANNUAL,
            0..0,
            0..1,
        )],
        &[],
        &[band(1, i32::MIN, MAX_ANNUAL - 1)],
        &[],
    );
    assert_eq!(out.balance.domain_outflow.0, 1);
    assert_eq!(
        out.lakes[0].geometric_volume.0,
        u128::from(u32::MAX) * 10_000
    );
}
