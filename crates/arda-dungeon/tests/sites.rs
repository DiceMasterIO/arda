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

use arda_dungeon::check::is_floor;
use arda_dungeon::site::{links, Link, LinkTo};
use arda_dungeon::{generate, Dungeon, ExitKind, Kind, Params};

fn chain(kind: Kind, n: u32) -> Vec<Dungeon> {
    (1..=n)
        .map(|level| {
            let (width, height) = LEVEL_SIZES[(level as usize - 1).min(2)];
            generate(&Params {
                seed: level_seed(42, 9, level, 0),
                kind,
                width,
                height,
            })
            .unwrap()
        })
        .collect()
}

fn way_up(ls: &[Link]) -> &Link {
    ls.iter().find(|l| l.kind != ExitKind::StairsDown).unwrap()
}

fn down(ls: &[Link]) -> &Link {
    ls.iter().find(|l| l.kind == ExitKind::StairsDown).unwrap()
}

#[test]
fn links_are_symmetric_and_end_sealed() {
    for kind in [Kind::Dungeon, Kind::Cave] {
        for n in 1..=3 {
            let levels = chain(kind, n);
            let all = links(&levels);
            assert_eq!(all.len(), levels.len());
            for (i, ls) in all.iter().enumerate() {
                assert_eq!(ls.len(), levels[i].exits.len());
                let up = way_up(ls);
                if i == 0 {
                    assert_eq!(up.to, LinkTo::Surface);
                } else {
                    let above = down(&all[i - 1]);
                    let level = u32::try_from(i).unwrap();
                    assert_eq!(
                        up.to,
                        LinkTo::Level {
                            level,
                            x: above.x,
                            y: above.y
                        }
                    );
                    assert_eq!(
                        above.to,
                        LinkTo::Level {
                            level: level + 1,
                            x: up.x,
                            y: up.y
                        }
                    );
                }
                if i + 1 == levels.len() {
                    assert_eq!(down(ls).to, LinkTo::Sealed);
                }
                for l in ls {
                    if let LinkTo::Level { level, x, y } = l.to {
                        let there = &levels[level as usize - 1];
                        assert!(is_floor(&there.layout, i64::from(x), i64::from(y)));
                    }
                }
            }
            let flat: Vec<&Link> = all.iter().flatten().collect();
            assert_eq!(flat.iter().filter(|l| l.to == LinkTo::Sealed).count(), 1);
            assert_eq!(flat.iter().filter(|l| l.to == LinkTo::Surface).count(), 1);
        }
    }
}

use arda_dungeon::site::sealed;
use arda_tactical::layout::{AssetRef, TAG_SECRET};
use arda_tactical::{render_with, Library, RenderOptions, Style, TacticalLayout, WallRole};
use std::path::Path;
use std::sync::OnceLock;

fn lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    })
}

fn pixels(layout: &TacticalLayout) -> Vec<u8> {
    let opts = RenderOptions {
        ppsq: 32,
        grid: false,
        lighting: true,
    };
    render_with(layout, lib(), 7, &opts, &Style::default())
        .unwrap()
        .data
}

fn ids(l: &TacticalLayout) -> Vec<String> {
    l.placements
        .iter()
        .filter_map(|p| match &p.asset {
            AssetRef::Id(id) => Some(id.clone()),
            AssetRef::Query { .. } => None,
        })
        .collect()
}

#[test]
fn the_sealed_layout_swaps_only_the_stairs_down() {
    let d = &chain(Kind::Dungeon, 1)[0];
    let s = sealed(&d.layout);
    s.check(lib()).unwrap();
    assert!(ids(&d.layout).iter().any(|i| i == "prop.stairs_down"));
    assert!(!ids(&s).iter().any(|i| i == "prop.stairs_down"));
    assert_eq!(
        ids(&s)
            .iter()
            .filter(|i| *i == "prop.stairs_sealed")
            .count(),
        1
    );
    assert_eq!(s.squares, d.layout.squares);
    assert_eq!(s.walls, d.layout.walls);
    assert_ne!(pixels(&s), pixels(&d.layout));
}

/// logic/18 §site-image: an unfound secret door is plain wall in every
/// tactical image, so a level image never gives one away.
#[test]
fn a_secret_door_is_painted_as_a_plain_run() {
    let d = (1..200_u64)
        .map(|seed| {
            generate(&Params {
                seed,
                kind: Kind::Dungeon,
                width: 48,
                height: 36,
            })
            .unwrap()
        })
        .find(|d| d.doors.iter().any(|g| g.secret))
        .unwrap();
    let mut plain = d.layout.clone();
    for w in &mut plain.walls {
        if w.kind == WallRole::Door && w.has_tag(TAG_SECRET) {
            w.kind = WallRole::Run;
            w.tags.retain(|t| t != TAG_SECRET);
        }
    }
    assert_ne!(plain.walls, d.layout.walls);
    assert_eq!(pixels(&plain), pixels(&d.layout));
}
