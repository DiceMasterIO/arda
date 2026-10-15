//! Recipe 9 (review 3.2-6): height above the nearest drainage also
//! measures to a lake surface or the sea.
use super::*;

fn run_with_water(f: &mut Fixture) {
    flow_metrics::run_with(&mut f.routing, &mut f.flow, &f.lakes, limits(), true).unwrap();
}

/// Recipes up to 8 leave every cell that drains into a lake or the sea
/// without a reference (12-21% of land carried the 65,535 dm sentinel);
/// recipe 9 anchors it to the standing water it drains into.
#[test]
fn hand_reaches_lakes_and_the_sea() {
    let mut f = bowl();
    f.run();
    assert_eq!(f.row(2).metrics.hand_at, None, "recipe 7 stops at the lake");
    let mut f = bowl();
    run_with_water(&mut f);
    let rim = f.row(2);
    let anchor = rim.metrics.hand_at.expect("recipe 9 anchors to the lake");
    assert!(
        f.row(anchor.raw()).lake.is_some(),
        "anchored on a lake cell"
    );
    assert_eq!(rim.metrics.hand_distance_mm, 100_000);
    // Lake cells themselves still carry no reference.
    assert_eq!(f.row(12).metrics.hand_at, None);
    // The outlet channel keeps its own reference.
    assert_eq!(f.row(14).metrics.hand_at, Some(f.at(14)));

    let sea = || {
        fixture_with_marine(
            2,
            &[5, 4, 2, 0],
            &[false, false, false, true],
            &[],
            &[(0, 40 * YEAR, 0, 0)],
            &[],
        )
    };
    let mut f = sea();
    f.run();
    assert_eq!(
        f.row(1).metrics.hand_at,
        None,
        "recipe 7 never reaches the sea"
    );
    let mut f = sea();
    run_with_water(&mut f);
    assert_eq!(
        f.row(2).metrics.hand_at,
        Some(f.at(3)),
        "recipe 9 reaches it"
    );
}
