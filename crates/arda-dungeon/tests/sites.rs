//! Site logic: level sizes, seeds, tiers, links and the sealed image.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_dungeon::site::{danger_tier, level_seed, level_sizes, LEVEL_SIZES, MAX_ATTEMPTS};

#[test]
fn level_counts_follow_the_odds_and_sizes_never_shrink() {
    let mut counts = [0_u32; 4];
    for key in 0..4000_u64 {
        let sizes = level_sizes(key.wrapping_mul(0x9E37_79B9_7F4A_7C15), 1);
        assert!((1..=3).contains(&sizes.len()));
        counts[sizes.len()] += 1;
        let area = |s: &(u32, u32)| s.0 * s.1;
        for pair in sizes.windows(2) {
            assert!(area(&pair[1]) >= area(&pair[0]), "{sizes:?}");
        }
        assert!(sizes.iter().all(|s| LEVEL_SIZES.contains(s)));
    }
    // 40 %, 40 %, 20 % within three points.
    for (n, want) in [(1, 1600_u32), (2, 1600), (3, 800)] {
        assert!(counts[n].abs_diff(want) < 120, "{n} levels: {}", counts[n]);
    }
}

#[test]
fn a_minimum_of_two_levels_is_kept() {
    for key in 0..500_u64 {
        assert!(level_sizes(key, 2).len() >= 2);
    }
}

#[test]
fn level_seeds_differ_by_level_and_attempt_and_are_stable() {
    let a = level_seed(42, 7, 1, 0);
    assert_eq!(a, level_seed(42, 7, 1, 0));
    assert_ne!(a, level_seed(42, 7, 2, 0));
    assert_ne!(a, level_seed(42, 7, 1, 1));
    assert_ne!(a, level_seed(43, 7, 1, 0));
    assert_ne!(a, level_seed(42, 8, 1, 0));
    assert_eq!(MAX_ATTEMPTS, 8);
}

#[test]
fn tiers_rise_by_one_per_level_and_stay_in_the_srd_bands() {
    assert_eq!(
        [danger_tier(1, 1), danger_tier(1, 2), danger_tier(1, 3)],
        [1, 2, 3]
    );
    assert_eq!(
        [danger_tier(2, 1), danger_tier(2, 2), danger_tier(2, 3)],
        [2, 3, 4]
    );
}
