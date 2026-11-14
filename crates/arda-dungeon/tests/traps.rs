//! Trap placement invariants (logic/18 §site-traps).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_dungeon::traps::{place, Trap, TrapKind};
use arda_dungeon::{generate, Dungeon, Kind, Params};
use arda_scene::{build_scene, Movement, Scene, Sq};
use arda_tactical::layout::EdgeAxis;
use arda_tactical::Library;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

fn lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    })
}

fn level(seed: u64, kind: Kind, (width, height): (u32, u32)) -> (Dungeon, Scene, Vec<Trap>) {
    let d = generate(&Params {
        seed,
        kind,
        width,
        height,
    })
    .unwrap();
    let scene = build_scene(&d.layout, lib(), seed, Some(&d.rules)).unwrap();
    let traps = place(&d, &scene, seed);
    (d, scene, traps)
}

fn sides(x: u32, y: u32, axis: EdgeAxis) -> [(i64, i64); 2] {
    let (x, y) = (i64::from(x), i64::from(y));
    match axis {
        EdgeAxis::Horizontal => [(x, y), (x, y - 1)],
        EdgeAxis::Vertical => [(x, y), (x - 1, y)],
    }
}

const SIZES: [(u32, u32); 3] = [(32, 24), (48, 36), (72, 56)];

#[test]
fn traps_lie_on_clear_floor_away_from_exits_and_doors() {
    let mut kinds = BTreeSet::new();
    for kind in [Kind::Dungeon, Kind::Cave] {
        for size in SIZES {
            for seed in 1..=12_u64 {
                let (d, scene, traps) = level(seed * 104_729, kind, size);
                let (lo, hi) = match size.0 {
                    32 => (1, 2),
                    48 => (2, 4),
                    _ => (3, 6),
                };
                assert!(
                    traps.len() <= hi,
                    "{kind:?} {size:?} {seed}: {}",
                    traps.len()
                );
                assert!(kind == Kind::Cave || traps.len() >= lo, "{size:?} {seed}");
                let door_sides: BTreeSet<(i64, i64)> = d
                    .doors
                    .iter()
                    .flat_map(|w| sides(w.x, w.y, w.axis))
                    .collect();
                let mut used = BTreeSet::new();
                for (n, t) in traps.iter().enumerate() {
                    assert_eq!(t.id, format!("t{}", n + 1));
                    kinds.insert(format!("{:?}", t.kind));
                    assert!(!t.squares.is_empty());
                    let own: BTreeSet<(i64, i64)> = t
                        .door
                        .map(|(x, y, a)| sides(x, y, a).into_iter().collect())
                        .unwrap_or_default();
                    for &(x, y) in t.squares.iter().chain(&t.affected) {
                        assert!(used.insert((x, y)), "square shared: {x},{y}");
                        assert_eq!(scene.movement_at(Sq(x, y)), Movement::Normal);
                        let at = (i64::from(x), i64::from(y));
                        assert!(!door_sides.contains(&at) || own.contains(&at));
                        for e in &d.exits {
                            let far = x.abs_diff(e.x).max(y.abs_diff(e.y));
                            assert!(far > 2, "{:?} {far} from an exit", t.kind);
                        }
                    }
                    match t.kind {
                        TrapKind::FallingNet | TrapKind::CollapsingRoof => {
                            assert_eq!(t.affected.len(), 4);
                        }
                        TrapKind::PoisonNeedle => {
                            let (x, y, a) = t.door.unwrap();
                            let door = d
                                .doors
                                .iter()
                                .find(|w| (w.x, w.y, w.axis) == (x, y, a))
                                .unwrap();
                            assert!(door.locked);
                            assert_eq!(kind, Kind::Dungeon);
                        }
                        _ => assert!(t.affected.is_empty() && t.door.is_none()),
                    }
                    assert!(!matches!(
                        t.kind,
                        TrapKind::FireBreathingStatue
                            | TrapKind::RollingSphere
                            | TrapKind::SphereOfAnnihilation
                    ));
                    if kind == Kind::Cave {
                        assert!(!matches!(
                            t.kind,
                            TrapKind::PoisonDarts | TrapKind::PoisonNeedle
                        ));
                    }
                }
            }
        }
    }
    assert_eq!(kinds.len(), 8, "{kinds:?}");
}

#[test]
fn trap_placement_is_deterministic() {
    let (_, _, a) = level(77, Kind::Dungeon, (48, 36));
    let (_, _, b) = level(77, Kind::Dungeon, (48, 36));
    assert_eq!(a, b);
    let (_, _, c) = level(78, Kind::Dungeon, (48, 36));
    assert_ne!(a, c);
}

#[test]
fn trap_kinds_serialise_as_the_srd_names() {
    assert_eq!(
        serde_json::to_string(&TrapKind::SpikedPit).unwrap(),
        "\"spiked_pit\""
    );
    assert_eq!(
        serde_json::to_string(&TrapKind::SphereOfAnnihilation).unwrap(),
        "\"sphere_of_annihilation\""
    );
}
