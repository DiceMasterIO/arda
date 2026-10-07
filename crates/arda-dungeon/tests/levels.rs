//! Dungeon and cave invariants: determinism, connectivity, doors that hang
//! between walls, stairs, sane scene rules, and pinned renders.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_dungeon::check::{floating_doors, is_floor, unreachable};
use arda_dungeon::{generate, Dungeon, ExitKind, Kind, Params, Purpose};
use arda_scene::{build_scene, Movement, Obscurement, Scene, WallKind};
use arda_tactical::layout::AssetRef;
use arda_tactical::{render_with, Library, RenderOptions, Style};
use std::path::Path;
use std::sync::OnceLock;

fn lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    })
}

fn params(seed: u64, kind: Kind, width: u32, height: u32) -> Params {
    Params {
        seed,
        kind,
        width,
        height,
    }
}

fn scene(d: &Dungeon) -> Scene {
    build_scene(&d.layout, lib(), d.params.seed, Some(&d.rules)).unwrap()
}

const SIZES: [(u32, u32); 4] = [(16, 16), (32, 24), (48, 36), (72, 56)];

fn each(mut f: impl FnMut(&Dungeon)) {
    for kind in [Kind::Dungeon, Kind::Cave] {
        for (w, h) in SIZES {
            for seed in 1..=6u64 {
                let d = generate(&params(seed * 7919 + u64::from(w), kind, w, h))
                    .unwrap_or_else(|e| panic!("{kind:?} {w}x{h} seed {seed}: {e}"));
                f(&d);
            }
        }
    }
}

#[test]
fn generation_is_deterministic() {
    for kind in [Kind::Dungeon, Kind::Cave] {
        let p = params(42, kind, 48, 36);
        let a = serde_json::to_string(&generate(&p).unwrap()).unwrap();
        let b = serde_json::to_string(&generate(&p).unwrap()).unwrap();
        assert_eq!(a, b, "{kind:?}");
        let c = serde_json::to_string(&generate(&params(43, kind, 48, 36)).unwrap()).unwrap();
        assert_ne!(a, c, "{kind:?}: seeds 42 and 43 agree");
    }
}

#[test]
fn layouts_check_against_the_placeholder_library() {
    each(|d| {
        d.layout
            .check(lib())
            .unwrap_or_else(|e| panic!("{}: {e}", d.layout.name));
        assert_eq!(d.rules.width, d.layout.width);
    });
}

#[test]
fn every_floor_square_is_reachable() {
    each(|d| {
        let s = scene(d);
        let start = d.exits.first().map(|e| (e.x, e.y)).unwrap();
        let lost = unreachable(&s, start);
        assert!(lost.is_empty(), "{}: unreachable {lost:?}", d.layout.name);
        // Every floor square is either walkable or under a blocking prop.
        let floor = (0..d.layout.height)
            .flat_map(|y| (0..d.layout.width).map(move |x| (x, y)))
            .filter(|&(x, y)| is_floor(&d.layout, i64::from(x), i64::from(y)));
        let walkable = floor
            .clone()
            .filter(|&(x, y)| {
                s.movement.0[(y * d.layout.width + x) as usize] != Movement::Impassable
            })
            .count();
        assert!(
            walkable * 10 >= floor.count() * 8,
            "{}: too cluttered",
            d.layout.name
        );
    });
}

#[test]
fn doors_hang_between_walls_and_floors() {
    each(|d| {
        let bad = floating_doors(&d.layout);
        assert!(bad.is_empty(), "{}: floating doors {bad:?}", d.layout.name);
    });
}

#[test]
fn levels_have_their_exits() {
    each(|d| {
        let has = |k: ExitKind| d.exits.iter().any(|e| e.kind == k);
        let placed = |id: &str| {
            d.layout
                .placements
                .iter()
                .any(|p| p.asset == AssetRef::Id(id.into()))
        };
        assert!(
            has(ExitKind::StairsDown) && placed("prop.stairs_down"),
            "{}",
            d.layout.name
        );
        match d.params.kind {
            Kind::Dungeon => assert!(has(ExitKind::StairsUp) && placed("prop.stairs")),
            Kind::Cave => assert!(has(ExitKind::Mouth)),
        }
        for e in &d.exits {
            assert!(is_floor(&d.layout, i64::from(e.x), i64::from(e.y)), "{e:?}");
        }
    });
}

#[test]
fn scene_rules_are_sane() {
    let (mut locked, mut secret, mut wade, mut swim, mut difficult) = (0, 0, 0, 0, 0);
    each(|d| {
        let s = scene(d);
        let w = d.layout.width;
        for y in 0..d.layout.height {
            for x in 0..w {
                let i = (y * w + x) as usize;
                if !is_floor(&d.layout, i64::from(x), i64::from(y)) {
                    assert_eq!(s.movement.0[i], Movement::Impassable, "rock ({x},{y})");
                    assert_eq!(s.obscured.0[i], Obscurement::Heavy, "rock ({x},{y})");
                }
                match s.movement.0[i] {
                    Movement::Wade => wade += 1,
                    Movement::Swim => swim += 1,
                    Movement::Difficult => difficult += 1,
                    _ => {}
                }
            }
        }
        for wall in &s.walls {
            if wall.kind.opens() {
                assert_eq!(wall.open, Some(false));
                assert!(wall.blocks_movement && wall.blocks_sight);
            } else {
                assert!(wall.blocks_movement, "{wall:?}");
            }
            locked += usize::from(wall.locked);
            secret += usize::from(wall.kind == WallKind::Secret);
        }
        if d.params.kind == Kind::Dungeon {
            assert_eq!(
                d.doors.len(),
                s.walls.iter().filter(|w| w.kind.opens()).count()
            );
        }
    });
    assert!(locked > 0 && secret > 0, "locked {locked} secret {secret}");
    assert!(wade > 0 && swim > 0, "wade {wade} swim {swim}");
    assert!(difficult > 0);
}

#[test]
fn dungeons_have_varied_rooms() {
    let d = generate(&params(7, Kind::Dungeon, 72, 56)).unwrap();
    let kinds: std::collections::BTreeSet<_> = d.rooms.iter().map(|r| r.purpose).collect();
    assert!(kinds.contains(&Purpose::Entrance));
    assert!(kinds.len() >= 5, "{kinds:?}");
    assert!(
        !d.layout.lights.is_empty()
            || d.layout
                .placements
                .iter()
                .any(|p| p.asset == AssetRef::Id("prop.torch_sconce".into()))
    );
}

#[test]
fn sizes_out_of_range_are_refused() {
    assert!(generate(&params(1, Kind::Cave, 8, 40)).is_err());
    assert!(generate(&params(1, Kind::Dungeon, 40, 400)).is_err());
}

/// Pinned render hashes, so art or generator drift is noticed. Rewrite
/// with `ARDA_BLESS_DUNGEON=1` after an intended change.
#[test]
fn renders_are_pinned() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/renders.txt");
    let mut lines = Vec::new();
    for (kind, seed) in [(Kind::Dungeon, 3), (Kind::Cave, 5)] {
        let d = generate(&params(seed, kind, 32, 24)).unwrap();
        let opts = RenderOptions {
            ppsq: 32,
            grid: false,
            lighting: true,
        };
        let img = render_with(&d.layout, lib(), seed, &opts, &Style::default()).unwrap();
        lines.push(format!(
            "{} {}x{} {}",
            d.layout.name,
            img.width,
            img.height,
            blake3::hash(&img.data)
        ));
    }
    let text = lines.join("\n") + "\n";
    if std::env::var_os("ARDA_BLESS_DUNGEON").is_some() {
        std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
        std::fs::write(&golden, &text).unwrap();
    }
    let want = std::fs::read_to_string(&golden)
        .unwrap_or_else(|e| panic!("{}: {e}; bless with ARDA_BLESS_DUNGEON=1", golden.display()));
    assert_eq!(
        text, want,
        "renders changed; bless with ARDA_BLESS_DUNGEON=1"
    );
}
