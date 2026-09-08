//! Regression controls for logic/01's boundary-classification correction.

use super::*;

fn plate(id: u8, centre: (i32, i32), drift: (i32, i32), crust: CrustType) -> Plate {
    Plate {
        id,
        centre_x: centre.0,
        centre_y: centre.1,
        drift_x: drift.0,
        drift_y: drift.1,
        crust,
    }
}

fn classify(a: &Plate, neighbours: [Plate; 4], reverse: bool) -> BoundaryMask {
    let mut entries = neighbours.each_ref();
    if reverse {
        entries.reverse();
    }
    boundary_kinds(a, entries)
}

fn rotate(p: Plate) -> Plate {
    Plate {
        centre_x: -p.centre_y,
        centre_y: p.centre_x,
        drift_x: -p.drift_y,
        drift_y: p.drift_x,
        ..p
    }
}

fn diagonal_tangent_pair() -> (Plate, Plate) {
    // The straight Voronoi interface is x + y = 0. Relative drift (-1, 1)
    // is exactly tangent to it; a raster staircase must not add forcing.
    (
        plate(0, (-4, -4), (0, 0), CrustType::Continental),
        plate(1, (4, 4), (-1, 1), CrustType::Continental),
    )
}

#[test]
fn tangential_diagonal_boundary_has_no_forcing() {
    let (a, b) = diagonal_tangent_pair();
    assert_eq!(classify(&a, [b, a, b, a], false), BoundaryMask::default());
}

#[test]
fn tangential_boundary_is_independent_of_neighbor_iteration() {
    let (a, b) = diagonal_tangent_pair();
    let neighbours = [b, a, b, a];
    assert_eq!(
        classify(&a, neighbours, false),
        classify(&a, neighbours, true)
    );
}

#[test]
fn diagonal_boundary_classification_survives_rigid_rotation() {
    let (a, b) = diagonal_tangent_pair();
    let original = classify(&a, [b, a, b, a], false);
    // Quarter-turn maps E,W,S,N to S,N,W,E. Rebuild the same geometric
    // neighbors in canonical E,W,S,N order after rotating the whole scene.
    let (a, b) = (rotate(a), rotate(b));
    assert_eq!(original, classify(&a, [a, b, b, a], false));
}

#[test]
fn a_tangential_pair_cannot_raise_or_lower_the_tectonic_field() {
    // This also checks that run_tectonics supplies the classifier's result to
    // every belt, despite the spatial warp of the two-plate ownership grid.
    let (mut a, mut b) = diagonal_tangent_pair();
    a.centre_x += 8;
    a.centre_y += 8;
    b.centre_x += 8;
    b.centre_y += 8;
    let field = run_tectonics(
        42,
        &[a, b],
        SimExtent {
            width: 16,
            height: 16,
        },
        1,
    );
    assert!(
        field.iter().all(|&height| height == 0),
        "tangential pair generated relief {}..{} mm",
        field.iter().min().unwrap(),
        field.iter().max().unwrap()
    );
}

#[test]
fn pair_kind_is_preserved_by_rotation_and_plate_exchange() {
    let a = plate(0, (0, 0), (0, 0), CrustType::Continental);
    let cases = [
        (plate(1, (4, 0), (-1, 0), a.crust), Boundary::Collision),
        (plate(1, (4, 0), (-1, 0), CrustType::Oceanic), Boundary::Arc),
        (plate(1, (4, 0), (1, 0), a.crust), Boundary::Rift),
    ];
    for (b, want) in cases {
        assert_eq!(classify(&a, [b, a, a, a], false), BoundaryMask(want as u8));
        assert_eq!(classify(&b, [b, a, b, b], false), BoundaryMask(want as u8));
        let (a, b) = (rotate(a), rotate(b));
        assert_eq!(classify(&a, [a, a, b, a], false), BoundaryMask(want as u8));
    }
    let (a, b) = diagonal_tangent_pair();
    assert_eq!(classify(&b, [b, a, b, a], false), BoundaryMask::default());
}

fn mixed_junction() -> (Plate, [Plate; 4]) {
    let a = plate(0, (0, 0), (0, 0), CrustType::Continental);
    (
        a,
        [
            plate(1, (4, 0), (-1, 0), a.crust),
            plate(2, (-4, 0), (1, 0), CrustType::Oceanic),
            plate(3, (0, 4), (0, 1), a.crust),
            a,
        ],
    )
}

#[test]
fn a_mixed_junction_retains_every_incident_kind() {
    let (a, neighbours) = mixed_junction();
    let result = classify(&a, neighbours, false);
    for want in [Boundary::Collision, Boundary::Arc, Boundary::Rift] {
        assert!(result.contains(want), "missing {want:?} from {result:?}");
    }
    assert_eq!(result, classify(&a, neighbours, true));
}

#[test]
fn mixed_junction_is_invariant_under_every_neighbor_permutation() {
    let (a, neighbours) = mixed_junction();
    let expected =
        BoundaryMask(Boundary::Collision as u8 | Boundary::Arc as u8 | Boundary::Rift as u8);
    let mut permutations = 0;
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                for l in 0..4 {
                    if i == j || i == k || i == l || j == k || j == l || k == l {
                        continue;
                    }
                    let entries = [i, j, k, l].map(|n| &neighbours[n]);
                    assert_eq!(boundary_kinds(&a, entries), expected);
                    permutations += 1;
                }
            }
        }
    }
    assert_eq!(permutations, 24);
}

#[test]
fn each_belt_sees_a_mixed_junction_as_its_own_distance_source() {
    let (a, neighbours) = mixed_junction();
    let mut kinds = [BoundaryMask::default(); 25];
    kinds[12] = classify(&a, neighbours, false);
    for want in [Boundary::Collision, Boundary::Arc, Boundary::Rift] {
        let distances = distance_to(&kinds, want, 5, 5, 3);
        assert_eq!(distances[12], 0, "{want:?} must originate at junction");
        assert_eq!(distances[13], 1);
        // Retain the existing Chebyshev belt metric, including diagonals.
        assert_eq!(distances[0], 2);
    }
}

#[test]
fn coincident_centres_and_same_plate_do_not_add_boundary_forcing() {
    let a = plate(0, (4, 4), (0, 0), CrustType::Continental);
    let b = plate(1, (4, 4), (1, -1), CrustType::Continental);
    assert_eq!(classify(&a, [a; 4], false), BoundaryMask::default());
    assert_eq!(classify(&a, [b; 4], false), BoundaryMask::default());
}

#[test]
fn normal_projection_handles_full_signed_coordinate_and_drift_differences() {
    let a = plate(
        0,
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        CrustType::Continental,
    );
    let b = plate(
        1,
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN),
        CrustType::Continental,
    );
    assert_eq!(
        classify(&a, [b, a, a, a], false),
        BoundaryMask(Boundary::Collision as u8)
    );
    assert_eq!(
        classify(&b, [b, a, b, b], false),
        BoundaryMask(Boundary::Collision as u8)
    );
}
